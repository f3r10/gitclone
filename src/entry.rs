use std::{fmt::Display, fs, path::PathBuf};

use anyhow::Result;

use crate::{
    util::{self, TreeEntry},
    Blob, Object, Tree,
};

#[derive(Eq, Clone, PartialEq, PartialOrd)]
pub struct Entry {
    pub name: String,
    pub oid: String,
    pub mode: String,
    pub path: Option<PathBuf>,
    pub blob: Blob,
}

impl Entry {
    // pub fn new(name: String, oid: String, mode: String) -> Self {
    //     Entry {
    //         name: name,
    //         oid: oid,
    //         mode,
    //         path: None,
    //     }
    // }

    pub fn build(path: PathBuf) -> Result<TreeEntry> {
        let metadata = fs::metadata(&path)?;
        let filetype = metadata.file_type();
        if filetype.is_file() {
            let blob = Blob::new(path.clone())?;
            let e = Entry {
                name: path
                    .file_name()
                    .expect("unable to get file name")
                    .to_str()
                    .expect("invalid filename")
                    .to_string(),
                oid: blob.clone().get_oid().to_string(),
                mode: util::get_mode(path.to_path_buf())?,
                path: Some(path.to_path_buf()),
                blob,
            };
            let entry = TreeEntry::TreeLeaf {
                entry: e,
                name: path
                    .file_name()
                    .expect("unable to get file name ")
                    .to_str()
                    .expect("invalid filename")
                    .to_string(),
            };
            Ok(entry)
        } else {
            Tree::build(path.clone())
        }
    }

    pub fn get_data(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let oid = hex::decode(&self.oid)?;

        data.extend_from_slice(self.mode.as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(self.name.as_bytes());
        data.push(0x00u8);
        data.extend_from_slice(&oid);
        Ok(data)
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &self.name))
    }
}
