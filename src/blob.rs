use core::fmt;
use std::{fmt::Display, path::PathBuf};

use anyhow::Result;

use crate::{util, Object};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Blob {
    pub pathbuf: PathBuf,
    type_: String,
    oid: Option<Vec<u8>>,
}

impl Object for Blob {
    fn get_data(&self) -> Result<Vec<u8>> {
        self.get_data_to_write()
    }

    fn type_(&self) -> &str {
        &self.type_
    }

    fn get_oid(&mut self) -> Result<Vec<u8>> {
        match &self.oid {
            Some(oid) => Ok(oid.to_vec()),
            None => {
                let digest = util::hexdigest_vec(&self.get_data_to_write()?);
                self.set_oid(&digest);
                Ok(digest)
            }
        }
    }
}

impl Blob {
    pub fn new(path_buf: PathBuf) -> Result<Blob> {
        Ok(Blob {
            pathbuf: path_buf.clone(),
            type_: "blob".to_string(),
            oid: None,
        })
    }

    pub fn get_content(&self) -> Result<Vec<u8>> {
        util::read_file(self.pathbuf.to_path_buf())
    }

    pub fn get_data_to_write(&self) -> Result<Vec<u8>> {
        let mut file_data = util::read_file(self.pathbuf.to_path_buf())?;
        let size = file_data.len().to_string();
        let mut data_to_write = String::new();
        data_to_write.push_str("blob");
        data_to_write.push(' ');
        data_to_write.push_str(&size);
        data_to_write.push('\0');
        let mut data_to_write = data_to_write.into_bytes();
        data_to_write.append(&mut file_data);
        Ok(data_to_write)
    }

    fn set_oid(&mut self, oid: &Vec<u8>) -> () {
        self.oid = Some(oid.to_vec());
    }
}

impl Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{}",
            &self
                .pathbuf
                .file_name()
                .expect("unable to get filename")
                .to_str()
                .expect("invalid filename")
                .to_string()
        ))
    }
}
