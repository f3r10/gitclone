use std::cell::Ref;
use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;

use crate::{
    util::{self, TreeAux},
    EntryAdd, Index, Tree,
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
        util::list_files(&self.pathname)?.into_iter().map(|p| {
            p.strip_prefix(&self.pathname).map(|p| p.to_path_buf()).map_err(|e| e.into())
        }).collect::<Result<Vec<_>, _>>()
    }

    pub fn list_dir<P, F>(&self, dirname: P, filter: F) -> Result<HashMap<PathBuf, Metadata>> 
        where 
            P: AsRef<Path>, 
            F: FnMut(&Result<DirEntry, std::io::Error>) -> bool,
        {
        let path = &self.pathname.join(dirname);
        let mut stats: HashMap<PathBuf, Metadata> = HashMap::new();
        let filter_entries = fs::read_dir(path)?
            .into_iter()
            .filter(filter);

        for er in filter_entries {
            let e = er?;
            let inner_path = e.path();
            let cmp = path.join(inner_path.to_path_buf());
            let relative = cmp.strip_prefix(&self.pathname)?;
            stats.insert(
                relative.to_path_buf(), 
                util::stat_file(&inner_path)?
            );
        };
        Ok(stats)
    }

    pub fn create_tree_from_index(&self, entries_add: Vec<Ref<EntryAdd>>) -> Result<TreeAux> {
        let mut root = TreeAux::new();
        for e in entries_add.into_iter() {
            let mut ancestors: Vec<_> = e
                .path
                .ancestors()
                .filter(|en| en.to_path_buf() != e.path && en.exists())
                .map(|e| e.to_path_buf())
                .collect();
            ancestors.reverse();
            root.add_entry(ancestors, e.path.to_path_buf(), Some(e.oid.to_vec()), e.get_mode()?)?;
        }
        Ok(root)
    }

    pub fn create_index_entry(&self, tree: &Tree, index: &mut Index) -> Result<()> {
        let mut work = tree.entries.clone();
        while let Some(entry) = work.pop() {
            if entry.is_tree() {
                work.append(&mut entry.entries.clone())
            } else {
                let stat = util::stat_file(&entry.path.canonicalize()?)?;
                index.add(entry.path.to_path_buf(), entry.sha1_hash.clone(), stat)?;
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

