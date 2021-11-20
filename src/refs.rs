use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use anyhow::Result;

pub struct Refs {
    pathname: PathBuf,
}

impl Refs {
    pub fn new(path_buf: &PathBuf) -> Self {
        Refs {
            pathname: path_buf.into(),
        }
    }

    pub fn update_head(&self, oid: String) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(&self.head_path())?;
        let res = file.write_all(&oid.as_bytes().to_vec())?;
        Ok(res)
    }

    pub fn read_head(&self) -> Option<String> {
        fs::read_to_string(&self.head_path()).ok()
    }

    pub fn head_path(&self) -> PathBuf {
        self.pathname.join("HEAD")
    }
}
