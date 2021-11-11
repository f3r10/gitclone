use std::{fs, path::PathBuf};

use anyhow::Result;

use crate::{Database, Entry, Tree, util::{self, TreeEntry}};

pub struct Workspace {
    pathname: PathBuf,
}

impl Workspace {
    pub fn new(path_buf: &PathBuf) -> Self {
        Workspace {
            pathname: path_buf.into(),
        }
    }

    pub fn build_root_tree(&self) -> Result<(Tree, String)> {
        let mut entries: Vec<TreeEntry> = fs::read_dir(&self.pathname)?
            .into_iter()
            .filter(|e| match e {
                Ok(p) => p.file_name() != ".git" && p.file_name() != "target",
                Err(_e) => true,
            })
            .flat_map(|e| e.map(|e| Entry::build(e.path())))
            .collect::<Result<Vec<_>>>()?;

        let entries_data =  util::get_data(&mut entries)?;

        let length = entries_data.len();

        let mut data = Vec::new();

        data.extend_from_slice("tree".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00u8);
        data.extend(entries_data);

        let data_to_write = data;

        let oid = util::hexdigest(&data_to_write);
        let tree = Tree::new(entries, self.pathname.clone(), oid.clone(), data_to_write);
        Ok((tree, oid))
    }

    pub fn persist_tree(&self, tree: &Tree, db: &Database) -> Result<()> {
        db.store(tree)?;
        for entry in &tree.entries {
            match entry.clone() {
                TreeEntry::TreeLeaf{entry: e, name: _} => {
                    db.store(&e.blob)?
                }
                TreeEntry::TreeBranch{tree, name: _} => {
                    self.persist_tree(&tree, db)?
                }
            }
        }
        Ok(())
    }

    pub fn get_git_path(&self) -> PathBuf {
        self.pathname.join(".git")
    }

    pub fn get_db_path(&self) -> PathBuf {
        self.get_git_path().join("objects")
    }
}
