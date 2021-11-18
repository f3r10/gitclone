use anyhow::Result;

use crate::{Database, Entry, EntryWrapper, Object, util::{self, TreeEntryAux}};
use core::fmt;
use std::{collections::HashMap, fmt::Display, path::PathBuf};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Tree {
    pub entries: Vec<EntryWrapper>,
    pub parent: PathBuf,
    type_: String,
    pub oid: Option<Vec<u8>>,
}

impl Object for Tree {
    fn get_data(&self) -> Result<Vec<u8>> {
        self.get_data_to_write()
    }

    fn type_(&self) -> &str {
        &self.type_
    }

    fn get_oid(&mut self) -> Result<Vec<u8>> {
         match &self.oid  {
             Some(oid) => Ok(oid.to_vec()),
             None => {
                 let digest = util::hexdigest_vec(&self.get_data_to_write()?);
                 self.set_oid(&digest);
                 Ok(digest)
             }
        }
    }
}

impl Tree {
    pub fn new(entries: Vec<EntryWrapper>, parent: PathBuf) -> Self {
        Tree {
            entries,
            type_: "tree".to_string(),
            oid: None,
            parent,
        }
    }

    fn set_oid(&mut self, oid: &Vec<u8>) -> () {
        self.oid = Some(oid.to_vec());
    }

    pub fn save_tree(&mut self, db: &Database) -> Result<()> {
        for e in self.entries.iter_mut() {
            match e {
                EntryWrapper::Entry { entry: _, name: _ } => {

                },
                EntryWrapper::EntryTree { tree: t, name: _ } => {
                    db.store(t)?
                },
            }
        }
        db.store(self)?;
        Ok(())
    }

    pub fn get_data_to_write(&self) -> Result<Vec<u8>> {
        let mut final_entries = self.entries.to_vec();
        let entries_data = util::get_data(&mut final_entries)?;
        let length = entries_data.len();

        let mut data = Vec::new();

        data.extend_from_slice("tree".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00u8);
        data.extend(entries_data);
        let data_to_write = data;
        Ok(data_to_write)
    }

    pub fn build_tree(root_path: PathBuf, entries: HashMap<PathBuf, TreeEntryAux>, db: &Database) -> Result<EntryWrapper> {

        let mut final_entries: Vec<EntryWrapper> = Vec::new();
        for (key, value) in entries {
            let entry = Entry::build_entry(key, value, db)?;
            final_entries.push(entry);
        };
        let tree = Tree::new(final_entries, root_path.to_path_buf());

        let tree_wrapper = EntryWrapper::EntryTree {
            tree: tree,
            name: root_path
                .file_name()
                .expect("unable to get filename")
                .to_str()
                .expect("invalid filename")
                .to_string(),
        };
        Ok(tree_wrapper)
    }
}
impl Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names: Vec<String> = Vec::new();

        for entry in &self.entries {
            match entry {
                EntryWrapper::Entry { entry: _, name } => names.push(name.to_string()),
                EntryWrapper::EntryTree { tree: _, name } => names.push(name.to_string()),
            }
        }

        let names = names.join("\n");

        f.write_fmt(format_args!("{}", names))
    }
}
