use anyhow::Result;

use crate::{
    util::{self, TreeEntry},
    Entry, Object,
};
use core::fmt;
use std::{fmt::Display, fs, path::PathBuf};

#[derive(Eq, Clone, PartialEq, PartialOrd)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
    pub parent: PathBuf,
    type_: String,
    pub oid: String,
    pub data_to_write: Vec<u8>,
}

impl Object for Tree {
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

impl Tree {
    pub fn new(entries: Vec<TreeEntry>, parent: PathBuf, oid: String, content: Vec<u8>) -> Self {
        Tree {
            entries,
            type_: "tree".to_string(),
            oid,
            parent,
            data_to_write: content,
        }
    }

    pub fn build(pathbuf: PathBuf) -> Result<TreeEntry> {
        let mut paths: Vec<PathBuf> = Vec::new();

        let mut dir = fs::read_dir(pathbuf.clone())?;
        while let Some(Ok(entry)) = dir.next() {
            let fpath = entry.path();
            if fpath.starts_with("./.git") {
                continue;
            }
            paths.push(fpath)
        }
        let mut entries: Vec<TreeEntry> = Vec::new();
        for path in paths {
            let entry = Entry::build(path)?;

            entries.push(entry);
        }

        let entries_data = util::get_data(&mut entries)?;

        let length = entries_data.len();

        let mut data = Vec::new();

        data.extend_from_slice("tree".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00u8);
        data.extend(entries_data);

        let data_to_write = data;

        let oid = util::hexdigest(&data_to_write);

        let t = Tree {
            entries,
            type_: "tree".to_string(),
            oid,
            parent: pathbuf.clone(),
            data_to_write,
        };
        let tree = TreeEntry::TreeBranch {
            tree: t,
            name: pathbuf
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
