use std::collections::HashMap;
use std::fs;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;

use crate::{
    util::{self, TreeAux},
    Database, Entry, EntryAdd, EntryWrapper, Index, Tree,
};

pub struct Workspace {
    pub pathname: PathBuf,
}

impl Workspace {
    pub fn new(path_buf: &PathBuf) -> Self {
        Workspace {
            pathname: path_buf.into(),
        }
    }

    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        let res = util::list_files(&self.pathname)?.iter().map(|e| {
            e.strip_prefix(&self.pathname).unwrap().to_path_buf()
        }).collect::<Vec<_>>();
        Ok(res)
    }

    pub fn list_dir<P: AsRef<Path>>(&self, dirname: P) -> Result<HashMap<PathBuf, Metadata>> {
        let path = &self.pathname.join(dirname);
        let mut stats: HashMap<PathBuf, Metadata> = HashMap::new();
        fs::read_dir(path)?
            .into_iter()
            .filter(|e| match e {
                Ok(p) => p.file_name() != ".git",
                Err(_e) => true,
            })
        //flat_map
        .for_each(|er| {
            er.map(|e| {
                let inner_path = e.path();
                let cmp = path.join(inner_path.to_path_buf());
                let relative = cmp.strip_prefix(&self.pathname).unwrap();
                stats.insert(
                    relative.to_path_buf(), 
                    util::stat_file(&inner_path).unwrap()
                    )
            }).unwrap();
        });
        // .collect::<Vec<_>>();
        Ok(stats)
    }

    pub fn create_tree_from_paths(&self, paths: Vec<PathBuf>) -> Result<TreeAux> {
        let paths = util::flatten_dot(paths)?;
        let mut e_add = Vec::new();
        for path in paths.clone().iter() {
            if path.is_dir() {
                let mut res = util::list_files(path)?;
                e_add.append(&mut res);
            } else {
                if path.exists() {
                    e_add.push(path.to_path_buf())
                } else {
                    return Err(anyhow!(format!("pathspec {:?} did not match any files", &path)))
                }
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
                    let stat = util::stat_file(&e.path.canonicalize()?)?;
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
