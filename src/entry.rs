use std::{fmt::Display, path::PathBuf};

use anyhow::Result;
use anyhow::anyhow;

use crate::{Blob, Database, Object, Tree, util::{self, TreeEntryAux}};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub enum EntryWrapper {
    EntryTree { tree: Tree, name: String },
    Entry { entry: Entry, name: String },
}

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Entry {
    pub name: String,
    pub oid: Option<Vec<u8>>,
    pub mode: String,
    pub path: PathBuf,
}

impl Entry {
    pub fn new(path: &PathBuf, oid: Option<Vec<u8>>) -> Result<Self> {
        let name = path
            .file_name()
            .expect("unable to get file name")
            .to_str()
            .expect("invalid filename")
            .to_string();
        let mode = util::get_mode(path.to_path_buf())?;
        Ok(Entry {
            name,
            oid,
            mode,
            path: path.to_path_buf(),
        })
    }
    pub fn build_entry(root_path: PathBuf, aux: TreeEntryAux, db: &Database) -> Result<EntryWrapper> {
        match aux {
            TreeEntryAux::TreeLeafAux { entry } => {
                let mut n_entry: Entry;
                if entry.oid.is_some() {
                    n_entry = entry.clone();
                } else {
                    let mut blob = Blob::new(entry.clone().path)?;
                    db.store(&mut blob)?;
                    n_entry = entry.clone();
                    n_entry.oid = Some(blob.get_oid()?);
                }
                
                let leaf = EntryWrapper::Entry {
                    entry: n_entry,
                    name: entry.name,
                };
                Ok(leaf)
            }
            TreeEntryAux::TreeBranchAux { tree } => {
                Tree::build_tree(root_path.clone(), tree.entries, db)
            }
        }
    }

    pub fn get_data(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        match &self.oid {
            Some(oid) => {
                data.extend_from_slice(self.mode.as_bytes());
                data.push(0x20u8);
                data.extend_from_slice(self.name.as_bytes());
                data.push(0x00u8);
                data.extend_from_slice(oid);
                Ok(data)
            },
            None => {
                Err(anyhow!("unable to get blob oid"))
            },
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &self.name))
    }
}
