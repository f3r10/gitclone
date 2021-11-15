use std::{fs::File, io::{BufReader, Read}};

use anyhow::Result;
use anyhow::anyhow;
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};

pub struct Checksum {
    reader: BufReader<File>,
    context: Context
}

const CHECKSUM_SIZE: usize = 20;


impl Checksum {
    pub fn new(file: File) -> Self {
        Checksum { 
            reader: BufReader::new(file),
            context: Context::new(&SHA1_FOR_LEGACY_USE_ONLY)
        }
    }

    pub fn read(&mut self, size: usize, update_context: bool) -> Result<Vec<u8>> {
        let mut f = vec![Default::default(); size];
        self.reader.read(&mut f[..])?;
        if update_context {
            self.context.update(&f);
        }
        Ok(f)
    }

    pub fn verify_checksum(mut self) -> Result<()> {
        let sum = self.read(CHECKSUM_SIZE, false)?;
        let digest = self.context.finish().as_ref().to_vec();  
        if sum != digest {
            return Err(anyhow!("Checksum does not match value stored on disk"))
        }
        Ok(())
    }
}
