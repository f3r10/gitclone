use anyhow::Result;

use crate::{Database, Entry, util::{self, TreeEntry, TreeEntryAux}};
use core::fmt;
use std::{collections::HashMap, fmt::Display, fs, path::PathBuf};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
    pub parent: PathBuf,
    type_: String,
    pub oid: Vec<u8>,
}

impl Tree {
    pub fn new(entries: Vec<TreeEntry>, parent: PathBuf, oid: &Vec<u8>) -> Self {
        Tree {
            entries,
            type_: "tree".to_string(),
            oid: oid.to_vec(),
            parent,
        }
    }

    pub fn build_tree(root_path: PathBuf, entries: HashMap<PathBuf, TreeEntryAux>, db: &Database) -> Result<TreeEntry> {

        // println!("tree - path: {:?}, paths: {:?}", pathbuf, paths);
        let mut final_entries: Vec<TreeEntry> = Vec::new();
        for (key, value) in entries {
            let entry = Entry::build_entry(key, value, db)?;
            final_entries.push(entry);
        };

        let entries_data = util::get_data(&mut final_entries)?;
        let length = entries_data.len();

        let mut data = Vec::new();

        data.extend_from_slice("tree".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00u8);
        data.extend(entries_data);
        let data_to_write = data;
        let oid = util::hexdigest_vec(&data_to_write);
        db.write_object(&oid, data_to_write)?;

        let tp = Tree {
            entries: final_entries,
            type_: "tree".to_string(),
            oid,
            parent: root_path.clone(),
        };
        let tree = TreeEntry::TreeBranch {
            tree: tp,
            name: root_path
                .file_name()
                .expect("unable to get filename")
                .to_str()
                .expect("invalid filename")
                .to_string(),
        };
        Ok(tree)
    }
}
impl Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names: Vec<String> = Vec::new();

        for entry in &self.entries {
            match entry {
                TreeEntry::TreeLeaf { entry: _, name } => names.push(name.to_string()),
                TreeEntry::TreeBranch { tree: _, name } => names.push(name.to_string()),
            }
        }

        let names = names.join("\n");

        f.write_fmt(format_args!("{}", names))
    }
}
