use std::os::unix::prelude::MetadataExt;
use std::{fmt::Display, path::PathBuf};

use anyhow::Result;

use crate::Object;
use crate::{Blob, Tree};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Entry {
    pub mode: String,
    pub name: String,
    pub sha1_hash: Vec<u8>,
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

impl Object for Entry {
    fn get_data(&self) -> Result<Vec<u8>> {
        Ok(self.data())
    }

    fn type_(&self) -> &str {
        self.mode.as_str()
    }

    fn get_oid(&mut self) -> Result<Vec<u8>> {
        Ok(self.sha1_hash.to_vec())
    }
}

impl Entry {
    pub fn new(
        mode: String,
        sha1_hash: Vec<u8>,
        path: PathBuf,
        name: String,
        entries: Vec<Entry>,
    ) -> Self {
        Self {
            mode,
            sha1_hash,
            path,
            name,
            entries,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let metadata = path.metadata()?;
        let filetype = metadata.file_type();

        let mut mode = String::new();
        // let mut sha1_hash: [u8; 20] = [0; 20];
        let mut sha1_hash = vec![];
        let mut entries: Vec<Entry> = Vec::new();

        if filetype.is_file() {
            let unix_mode = metadata.mode();
            let is_executable = (unix_mode & 0o001) != 0;
            if is_executable {
                mode.push_str("100755");
            } else {
                mode.push_str("100644")
            }
            let mut blob = Blob::new(path.clone())?;
            // db.store(&mut blob);
            sha1_hash = blob.get_oid()?;
        } else if filetype.is_symlink() {
            mode.push_str("120000");
            let mut blob = Blob::new(path.clone())?;
            // db.store(&mut blob);
            sha1_hash = blob.get_oid()?;
        } else if filetype.is_dir() {
            mode.push_str("040000");
            let tree = Tree::new(path.clone())?;
            entries = tree.entries;
            sha1_hash = tree.sha1_hash;
        }

        let name = path
            .file_name()
            .expect("Expected a name")
            .to_str()
            .expect("Invalif filename")
            .to_string();

        Ok(Self {
            mode,
            name,
            sha1_hash,
            path: path.to_path_buf(),
            entries,
        })
    }

    pub fn data(&self) -> Vec<u8> {
        let mut data = Vec::new();

        data.extend_from_slice(self.mode.as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(self.name.as_bytes());
        data.push(0x00u8);
        data.extend_from_slice(&self.sha1_hash);

        data
    }

    pub fn is_tree(&self) -> bool {
        self.mode == "040000"
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &self.name))
    }
}
