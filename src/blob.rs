use std::{io::{Cursor, Read}, path::PathBuf};

use anyhow::Result;

use crate::{util, Object};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Blob2 {
    pub pathbuf: PathBuf,
    type_: String,
    oid: Vec<u8>,
}

pub struct Blob {
    content: Vec<u8>
}

impl Blob {
    pub fn new(content: Vec<u8>) -> Result<Self> {
        Ok(Blob{
            content
        })
    }

    pub fn parse(cursor: &mut Cursor<Vec<u8>>) -> Result<Self> {
        let mut message = vec![];
        cursor.read_to_end(&mut message)?;
        Blob::new(message)
    }

}

impl Object for Blob {
    fn get_data(&self) -> Result<Vec<u8>> {
        Ok(self.content.to_vec())
    }

    fn type_(&self) -> &str {
        "blob"
    }

    fn get_oid(&self) -> Result<Vec<u8>> {
        let digest = util::hexdigest_vec(&self.content);
        Ok(digest)
    }
}
