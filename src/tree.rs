use anyhow::anyhow;
use anyhow::Result;

use crate::util::TreeAux;
use crate::Entry;
use crate::{
    util::{self, TreeEntryAux},
    Database, Object,
};
use std::path::Path;
use std::{
    io::{BufRead, Cursor, Read},
    path::PathBuf,
};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub struct Tree {
    pub entries: Vec<Entry>,
    pub sha1_hash: Vec<u8>,
}

pub fn build_add_tree(root: TreeAux) -> Result<Tree> {
    let mut entries = Vec::new();
    for (entry, aux) in root.entries {
        match aux {
            TreeEntryAux::TreeLeafAux { entry } => entries.push(entry),
            TreeEntryAux::TreeBranchAux { tree } => {
                let tree = build_add_tree(tree)?;
                let name = entry
                    .file_name()
                    .expect("Expected a name")
                    .to_str()
                    .expect("Invalif filename")
                    .to_string();
                let tree_entry = Entry::new(
                    "040000".to_string(),
                    tree.sha1_hash,
                    entry.to_path_buf(),
                    name,
                    tree.entries,
                );
                entries.push(tree_entry)
            }
        }
    }

    Tree::new_with_entries(entries)
}

impl Object for Tree {
    fn get_data(&self) -> Result<Vec<u8>> {
        self.get_data_to_write()
    }

    fn type_(&self) -> &str {
        "tree"
    }

    fn get_oid(&self) -> Result<Vec<u8>> {
        Ok(self.sha1_hash.to_vec())
    }
}

impl Tree {
    pub fn new_with_entries(entries: Vec<Entry>) -> Result<Self> {
        let mut entries = entries;

        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let mut entries_data = vec![];
        for entry in &entries.clone() {
            entries_data.extend(entry.data());
        }

        let length = entries_data.len();

        let mut data = vec![];

        data.extend_from_slice("tree".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00u8);
        data.extend(entries_data);

        let sha1_hash = util::hexdigest_vec(&data);

        let tree = Tree { entries, sha1_hash };

        Ok(tree)
    }

    pub fn new(path: PathBuf, db: &Database) -> Result<Self> {
        let mut paths: Vec<PathBuf> = vec![];
        let mut dir = std::fs::read_dir(path)?;
        while let Some(Ok(entry)) = dir.next() {
            let fpath = entry.path();
            if fpath.starts_with(".git") {
                continue;
            }
            paths.push(fpath)
        }

        let mut entries: Vec<Entry> = vec![];
        for path in paths {
            let entry = Entry::from_file(path, db)?;
            entries.push(entry);
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let mut entries_data = vec![];
        for entry in &entries.clone() {
            entries_data.extend(entry.data());
        }

        let length = entries_data.len();

        let mut data = vec![];

        data.extend_from_slice("tree".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00u8);
        data.extend(entries_data);

        let sha1_hash = util::hexdigest_vec(&data);

        let tree = Tree { entries, sha1_hash };

        Ok(tree)
    }

    pub fn save_tree(&mut self, db: &Database) -> Result<()> {
        for e in self.entries.iter_mut() {
            if e.is_tree() {
                let mut tree = Tree::new_with_entries(e.entries.clone())?;
                tree.save_tree(&db)?;
            }
        }
        db.store(self)?;
        Ok(())
    }

    pub fn get_data_to_write(&self) -> Result<Vec<u8>> {
        let mut final_entries = self.entries.to_vec();
        final_entries.sort_by(|a, b| a.name.cmp(&b.name));

        let mut entries_data = vec![];
        for entry in &final_entries.clone() {
            entries_data.extend(entry.data());
        }

        let length = entries_data.len();

        let mut data = vec![];

        data.extend_from_slice("tree".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00u8);
        data.extend(entries_data);
        Ok(data)
    }

    pub fn parse(cursor: &mut Cursor<Vec<u8>>, sha1_hash: Vec<u8>) -> Result<Self> {
        let mut entries = vec![];
        loop {
            let mut mode = vec![];
            let num_read = cursor.read_until(0x20u8, &mut mode)?;
            if num_read == 0 {
                break;
            }
            let mode = String::from_utf8(mode)?.trim().to_string();
            // println!("mode: {:?}", mode.trim());

            let mut name = vec![];
            cursor.read_until(0x00, &mut name)?;
            let mut name = String::from_utf8(name.to_vec())?;
            // println!("name: {:?}", name);
            name.pop();
            // println!("name after pop: {:?}", name);

            let mut sha1_hash: [u8; 20] = [0; 20];
            cursor.read_exact(&mut sha1_hash)?;
            // println!("hash: {:?}", sha1_hash);
            let entry = Entry::new(mode, sha1_hash.to_vec(), Path::new("").to_path_buf(), name, vec![]);
            entries.push(entry)
        }
        Ok(Self{
            entries,
            sha1_hash
        })

    }

    pub fn new_from_files(paths: Vec<PathBuf>, db: &Database) -> Result<Self> {
        let paths = util::flatten_dot(paths)?;
        let mut root_tree_entries = vec![];
        for path in &paths {
            if !path.exists() {
                return Err(anyhow!(format!(
                    "pathspec {:?} did not match any files",
                    &path
                )));
            }
            let entry = Entry::from_file(path.to_path_buf(), db)?;
            root_tree_entries.push(entry);
        }
        Tree::new_with_entries(root_tree_entries)
    }
}
