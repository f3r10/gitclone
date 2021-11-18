use std::{fmt::Display, fs::{self}, path::PathBuf};

use anyhow::Result;

use crate::{Blob, Database, Object, Tree, util::{self, TreeEntry, TreeEntryAux}};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Entry {
    pub name: String,
    pub oid: String,
    pub mode: String,
    pub path: Option<PathBuf>,
    // pub blob: Blob,
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
    pub fn build_entry(root_path: PathBuf, aux: TreeEntryAux, db: &Database) -> Result<TreeEntry> {
        // println!("path: {:?} is_dir: {:?}, paths: {:?}", path, path.is_dir(), paths);
        match aux {
            TreeEntryAux::TreeLeafAux { entry } => {
                let blob = Blob::new(entry.path.clone())?;
                // db.write_object(blob.get_oid().to_string(), blob.get_data())?;
                let e = Entry {
                    name: entry.path
                        .file_name()
                        .expect("unable to get file name")
                        .to_str()
                        .expect("invalid filename")
                        .to_string(),
                        oid: util::encode_vec(&blob.clone().get_oid()?),
                        mode: util::get_mode(entry.path.to_path_buf())?,
                        path: Some(entry.path.to_path_buf()),
                        // blob,
                };
                let entry = TreeEntry::TreeLeaf {
                    entry: e,
                    name: entry.path
                        .file_name()
                        .expect("unable to get file name ")
                        .to_str()
                        .expect("invalid filename")
                        .to_string(),
                };
                Ok(entry)
            }
            TreeEntryAux::TreeBranchAux { tree } => {
                Tree::build_tree(root_path.clone(), tree.entries, db)
            }
        }
    }

    pub fn build(path: PathBuf, db: &Database) -> Result<TreeEntry> {
        let metadata = fs::metadata(&path)?;
        let filetype = metadata.file_type();
        if filetype.is_file() {
            let mut blob = Blob::new(path.clone())?;
            db.store(&mut blob)?;
            // db.write_object(util::encode_vec(&blob.get_oid()?), blob.get_data()?)?;
            let e = Entry {
                name: path
                    .file_name()
                    .expect("unable to get file name")
                    .to_str()
                    .expect("invalid filename")
                    .to_string(),
                oid: util::encode_vec(&blob.clone().get_oid()?),
                mode: util::get_mode(path.to_path_buf())?,
                path: Some(path.to_path_buf()),
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
            Tree::build(path.clone(), db)
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
