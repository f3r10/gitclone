use std::collections::BTreeSet;
use std::path::Path;
use std::{fs, path::PathBuf};

use anyhow::anyhow;
use anyhow::Result;
use chrono::Local;

use crate::util;
use crate::Author;
use crate::Commit;
use crate::Object;
use crate::Refs;
use crate::{Database, Index, Workspace};

pub struct Command {
    workspace: Workspace,
    db: Database,
    pub index: Index,
}

pub struct Status {
    untracked: BTreeSet<String>
}

impl Status {
    pub fn new() -> Self {
        Status {
            untracked: BTreeSet::new()
        }
    }
    pub fn run(&mut self, cmd: &mut Command) -> Result<()> {
        if !cmd.workspace.get_git_path().exists() {
            return Err(anyhow!("not a git repository (or any parent up to mount point /)"))
        }
        if cmd.workspace.get_git_path().join("index").exists() {
            cmd.index.load()?;
        }
        self.scan_workspace(None, cmd)?;
        self.untracked.iter().for_each(|e| {
            println!("?? {}", e)
        });
        Ok(())

    }

    pub fn scan_workspace(&mut self, prefix: Option<PathBuf> , cmd: &mut Command) -> Result<()> {
        let prefix = prefix.unwrap_or(Path::new("").to_path_buf());
        for (key, value) in cmd.workspace.list_dir(prefix)?.iter() {
            if cmd.index.is_tracked(key.to_path_buf()) {
                if value.is_dir() {
                    self.scan_workspace(Some(key.to_path_buf()), cmd)?;
                }
            } else {
                let final_name = if value.is_dir() {
                    format!("{}/", key.to_str().unwrap().to_string())
                } else {
                    key.to_str().unwrap().to_string()
                };
                self.untracked.insert(final_name);
            }
        };
        Ok(())
    }
}

impl Command {
    pub fn new(path_buf: PathBuf) -> Result<Self> {
        let ws = Workspace::new(&path_buf);
        let db = Database::new(&path_buf.join(".git/objects"));
        let index = Index::new(&path_buf.join(".git/index"));
        Ok(Command {
            workspace: ws,
            db,
            index,
        })
    }

    pub fn status(&mut self) -> Result<()> {
        let mut status = Status::new();
        status.run(self)
    }
    pub fn init(&self) -> Result<()> {
        let git_path = &self.workspace.get_git_path();
        for dir in ["objects", "refs"] {
            fs::create_dir_all(git_path.join(dir))?
        }

        println!(
            "Initialized empty Jit repository in: {:?}",
            git_path.to_str()
        );
        Ok(())
    }

    pub fn add(&mut self, paths: Vec<PathBuf>) -> Result<()> {
        if !self.workspace.get_git_path().exists() {
            return Err(anyhow!("not a git repository (or any parent up to mount point /)"))
        }
        if self.workspace.get_git_path().join("index").exists() {
            self.index.load()?;
        }
        let root = self.workspace.create_tree_from_paths(paths)?;
        let tree = &self.workspace.build_add_tree(root, &self.db)?;
        self.workspace
            .create_index_entry(&tree, &self.db, &mut self.index)?;
        self.index.write_updates()?;
        Ok(())
    }

    pub fn commit(&mut self, author: &str, email: &str, message: &str) -> Result<()> {
        if self.workspace.get_git_path().join("index").exists() {
            self.index.load()?;
        } else {
            return Err(anyhow!("Unable to commit if there is not a index file"));
        }
        let entries = self.index.each_entry()?;
        let root = self.workspace.create_tree_from_index(entries)?;
        let mut tree = self.workspace.build_add_tree(root, &self.db)?;
        tree.save_tree(&self.db)?;
        match tree.oid {
            Some(oid) => {
                let refs = Refs::new(&self.workspace.get_git_path());
                let parent = refs.read_head();
                let current_time = Local::now();
                let author = Author::new(author, email, current_time);
                let oid  = util::encode_vec(&oid);
                let mut commit = Commit::new(
                    &oid,
                    author,
                    message.to_string(),
                    parent.clone(),
                );
                self.db.store(&mut commit)?;
                refs.update_head(util::encode_vec(&commit.get_oid()?))?;
                let is_root = if parent.is_none() {
                    "(root-commit)"
                } else {
                    ""
                };
                println!(
                    "[{} {:?}] {:?}",
                    is_root,
                    util::encode_vec(&commit.get_oid()?),
                    message.lines().next().unwrap()
                );
                Ok(())
            }
            None => {
                Err(anyhow!("Unable to execute the commit command because the root tree does not have a valid oid"))
            }
        }
    }
}
