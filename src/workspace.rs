use std::{fs, path::PathBuf};

use anyhow::anyhow;
use anyhow::Result;

use crate::{
    util::{self, TreeAux},
    Database, Entry, EntryAdd, EntryWrapper, Index, Tree,
};

pub struct Workspace {
    pathname: PathBuf,
}

impl Workspace {
    pub fn new(path_buf: &PathBuf) -> Self {
        Workspace {
            pathname: path_buf.into(),
        }
    }

    pub fn list_files(&self, path: &PathBuf) -> Result<Vec<PathBuf>> {
        let res = fs::read_dir(path)?
            .into_iter()
            .filter(|e| match e {
                Ok(p) => p.file_name() != ".git" && p.file_name() != "target",
                Err(_e) => true,
            })
            .flat_map(|er| {
                er.map(|e| {
                    let inner_path = e.path();
                    if inner_path.is_dir() {
                        self.list_files(&inner_path)
                    } else {
                        Ok(vec![inner_path])
                    }
                })
            })
            .flatten()
            .flatten()
            .collect::<Vec<_>>();
        Ok(res)
    }

    pub fn create_tree_from_paths(&self, paths: Vec<PathBuf>) -> Result<TreeAux> {
        let paths = util::flatten_dot(paths)?;
        let mut e_add = Vec::new();
        for path in paths.clone().iter() {
            if path.is_dir() {
                let mut res = self.list_files(path)?;
                e_add.append(&mut res);
            } else {
                e_add.push(path.to_path_buf())
            }
        }

        let mut root = TreeAux::new();
        for e in e_add.into_iter() {
            let mut ancestors: Vec<_> = e
                .ancestors()
                .filter(|en| en.to_path_buf() != e && en.exists())
                .map(|e| e.to_path_buf())
                .collect();
            ancestors.reverse();
            root.add_entry(ancestors, e, None)?;
        }
        Ok(root)
    }

    pub fn create_tree_from_index(&self, entries_add: Vec<&EntryAdd>) -> Result<TreeAux> {
        let mut root = TreeAux::new();
        for e in entries_add.into_iter() {
            let mut ancestors: Vec<_> = e
                .path
                .ancestors()
                .filter(|en| en.to_path_buf() != e.path && en.exists())
                .map(|e| e.to_path_buf())
                .collect();
            ancestors.reverse();
            root.add_entry(ancestors, e.path.to_path_buf(), Some(e.oid.to_vec()))?;
        }
        Ok(root)
    }

    pub fn build_add_tree(&self, root: TreeAux, db: &Database) -> Result<Tree> {
        let mut entries = Vec::new();
        for (entry, aux) in root.entries {
            let t = Entry::build_entry(entry, aux, db)?;
            entries.push(t)
        }

        let tree = Tree::new(entries, self.pathname.clone());
        Ok(tree)
    }

    pub fn create_index_entry(&self, tree: &Tree, db: &Database, index: &mut Index) -> Result<()> {
        for entry in &tree.entries {
            match entry {
                EntryWrapper::Entry { entry: e, name: _ } => {
                    let stat = util::stat_file(e.path.canonicalize()?)?;
                    match &e.oid {
                        Some(oid) => {
                            index.add(e.path.to_path_buf(), oid.to_vec(), stat)?;
                        }
                        None => {
                            anyhow!("Unable to build entry because there is not a valid oid");
                        }
                    }
                }
                EntryWrapper::EntryTree { tree, name: _ } => {
                    self.create_index_entry(&tree, db, index)?
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
