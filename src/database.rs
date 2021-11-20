use anyhow::Result;
use libflate::zlib::Encoder;
use std::fs::OpenOptions;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use crate::{util, Object};

pub struct Database {
    pathname: PathBuf,
}

impl Database {
    pub fn new(path_buf: &PathBuf) -> Self {
        Database {
            pathname: path_buf.into(),
        }
    }

    pub fn store(&self, object: &mut dyn Object) -> Result<()> {
        let data = object.get_data()?;
        self.write_object(&object.get_oid()?, data)
    }

    pub fn write_object(&self, oid: &Vec<u8>, content: Vec<u8>) -> Result<()> {
        let oid_s = util::encode_vec(&oid);
        let (a, b) = oid_s.split_at(2);
        let path = &self.pathname.join(a);
        if !path.exists() {
            fs::create_dir_all(&path).expect("unable to create path");
            let file_content = path.join(b);
            let mut file = OpenOptions::new()
                .read(true)
                .create(true)
                .write(true)
                .open(&file_content)?;

            let mut encoder = Encoder::new(Vec::new())?;
            io::copy(&mut &content[..], &mut encoder)?;
            let encode_data = encoder.finish().into_result()?;

            file.write_all(&encode_data)?
        }
        Ok(())
    }
}
