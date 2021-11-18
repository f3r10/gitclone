use std::{fs, path::{Path, PathBuf}};

use anyhow::Result;

use crate::{Database, Entry, Index, Tree, util::{self, TreeEntry}};

pub struct Workspace {
    pathname: PathBuf,
}

impl Workspace {
    pub fn new(path_buf: &PathBuf) -> Self {
        Workspace {
            pathname: path_buf.into(),
        }
    }

    pub fn build_add_tree(&self, paths: Vec<PathBuf>, db: &Database) -> Result<Tree> {
        let mut entries = Vec::new();
        for path in paths.iter() {
            if path.is_dir() {
                let mut dir_entries: Vec<TreeEntry> = fs::read_dir(path)?
                    .into_iter()
                    .filter(|e| match e {
                        Ok(p) => p.file_name() != ".git" && p.file_name() != "target",
                        Err(_e) => true,
                    })
                    .flat_map(|e| e.map(|e| Entry::build(e.path(), db)))
                    .collect::<Result<Vec<_>>>()?;
                entries.append(&mut dir_entries);
            } else {
                let entry = Entry::build(path.to_path_buf(), db)?;
                entries.push(entry);
            }
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

        let oid = util::hexdigest_vec(&data_to_write);
        let tree = Tree::new(entries, self.pathname.clone(), oid);
        // db.write_object(oid.clone(), data_to_write)?;
        Ok(tree)
    }

    pub fn create_index_entry(&self, tree: &Tree, db: &Database, index: &mut Index) -> Result<()> {
        for entry in &tree.entries {
            match entry {
                TreeEntry::TreeLeaf { entry: e, name: _ } => {
                    let stat = util::stat_file(e.path.canonicalize()?)?;
                    index.add(e.path.to_path_buf(), e.oid.clone(), stat)?;
                } ,
                TreeEntry::TreeBranch { tree, name: _ } => self.create_index_entry(&tree, db, index)?,
            }
        }
        Ok(())
    }

    pub fn build_root_tree(&self, pathname: Option<PathBuf>, db: &Database) -> Result<(Tree, String)> {
        let pathname = pathname.unwrap_or(self.pathname.clone());
        let mut entries = Vec::new();
        if pathname.is_dir() {
            let mut dir_entries: Vec<TreeEntry> = fs::read_dir(pathname)?
                .into_iter()
                .filter(|e| match e {
                    Ok(p) => p.file_name() != ".git" && p.file_name() != "target",
                    Err(_e) => true,
                })
                .flat_map(|e| e.map(|e| Entry::build(e.path(), db)))
                .collect::<Result<Vec<_>>>()?;
            entries.append(&mut dir_entries);
        } else {
            let entry = Entry::build(pathname, db)?;
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

        let oid = util::hexdigest_vec(&data_to_write);
        let tree = Tree::new(entries, self.pathname.clone(), oid.clone());
        db.write_object(&oid, data_to_write)?;
        Ok((tree, util::encode_vec(&oid)))
    }

    pub fn get_git_path(&self) -> PathBuf {
        self.pathname.join(".git")
    }

    pub fn get_db_path(&self) -> PathBuf {
        self.get_git_path().join("objects")
    }
}
