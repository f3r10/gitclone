use core::fmt;
use std::{fmt::Display, path::PathBuf};

use anyhow::Result;

use crate::{Object, util};


#[derive(Eq, Clone, PartialEq, PartialOrd)]
pub struct Blob {
    pathbuf: PathBuf,
    type_: String,
    oid: String,
    content: Vec<u8>,
    data_to_write: Vec<u8>
}

impl Object for Blob {
    fn get_data(&self) -> Vec<u8> {
        self.data_to_write.clone()
    }

    fn type_(&self) -> &str {
        &self.type_
    }

    // fn set_oid(&mut self, oid: String) {
    //     self.oid = oid;
    // }

    fn get_oid(&self) -> &str {
        &self.oid
    }
}

impl Blob {

    pub fn new(path_buf: PathBuf) -> Result<Blob> {
        let mut file_data = util::read_file(path_buf.clone())?;
        let size = file_data.len().to_string();
        let content = file_data.clone();
        let mut data_to_write = String::new();
        data_to_write.push_str("blob");
        data_to_write.push(' ');
        data_to_write.push_str(&size);
        data_to_write.push('\0');
        let mut data_to_write = data_to_write.into_bytes();
        data_to_write.append(&mut file_data);
        let digest = util::hexdigest(&data_to_write);
        Ok(Blob {
            pathbuf : path_buf.clone(),
            type_ : "blob".to_string(),
            oid: digest,
            content,
            data_to_write
        })
    }
}

impl Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", &self.pathbuf.file_name().expect("unable to get filename").to_str().expect("invalid filename").to_string()))
    }
}
