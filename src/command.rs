use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::fs::{DirEntry, Metadata};
use std::path::Path;
use std::rc::Rc;
use std::{fs, path::PathBuf};

use anyhow::anyhow;
use anyhow::Result;
use chrono::Local;

use crate::{Blob, util};
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
    stat: HashMap<String, Metadata>,
    untracked: BTreeSet<String>,
    changed: BTreeSet<String>,
    cmd: Rc<RefCell<Command>>
}

impl Status {
    pub fn new(cmd: Command) -> Self {
        Status {
            stat: HashMap::new(),
            untracked: BTreeSet::new(),
            changed: BTreeSet::new(),
            cmd: Rc::new(RefCell::new(cmd))
        }
    }
    pub fn run(&mut self) -> Result<()> {
        let cmd = self.cmd.clone();
        if !cmd.borrow().workspace.get_git_path().exists() {
            return Err(anyhow!("not a git repository (or any parent up to mount point /)"))
        }
        if cmd.borrow().workspace.get_git_path().join("index").exists() {
            cmd.borrow_mut().index.load()?;
        }
        self.scan_workspace(None)?;
        self.detect_workspace_changes()?;
        cmd.borrow().index.write_updates()?;
        self.changed.iter().for_each(|e| {
            println!(" M {}", e)
        });
        self.untracked.iter().for_each(|e| {
            println!("?? {}", e)
        });
        Ok(())

    }

    pub fn scan_workspace(&mut self, prefix: Option<PathBuf>) -> Result<()> {
        let cmd = self.cmd.borrow();
        let prefix = prefix.unwrap_or(Path::new("").to_path_buf());
        let e = |e: &Result<DirEntry, std::io::Error>| match e {
                Ok(p) => p.file_name() != ".git",
                Err(_e) => true,
            };
        let mut work = vec![prefix];
        while let Some(dir) = work.pop() {
            for (key, value) in cmd.workspace.list_dir(dir, e)?.iter() {
                if cmd.index.is_tracked(key.to_path_buf()) {
                    if value.is_dir() {
                        work.push(key.to_path_buf())
                    }
                    if value.is_file() {
                        self.stat.insert(key.display().to_string(), value.clone());
                    }
                } else {
                    if self.any_trackable_file(key.to_path_buf(), value)? {
                        let final_name = if value.is_dir() {
                            format!("{}/", key.display())
                        } else {
                            key.display().to_string()
                        };
                        self.untracked.insert(final_name);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn any_trackable_file(&self, path: PathBuf, stat: &Metadata) -> Result<bool> {
        let cmd = self.cmd.borrow();
        let e = |e: &Result<DirEntry, std::io::Error>| match e {
                Ok(p) => p.file_name() != ".git",
                Err(_e) => true,
            };

        // if there is any file that can be tracked the function should stop without needing to
        // check the rest files or directories -- not going further and checking possible
        // directories will make this function faster. 
        let mut work = vec![(path.to_path_buf(), stat.to_owned())];
        let mut res: bool = false;
        while let Some((dir, stat)) = work.pop() {
            if stat.is_file() {
                res = !cmd.index.is_tracked(path.to_path_buf());
                break;
            }

            if !stat.is_dir() {
                res = false;
                break;
            } else {
                let items = cmd.workspace.list_dir(dir.to_path_buf(), e)?;
                let iter = items.into_iter();
                let (files, dirs): (Vec<_>, Vec<_>) = iter.partition(|(_, item_stat)| item_stat.is_file());
                work.extend(dirs);
                work.extend(files);
            }
        }
        Ok(res)
    }

    pub fn detect_workspace_changes(&mut self) -> Result<()> {
        let cmd = self.cmd.clone();
        let borrow = &mut *cmd.borrow_mut();
        let mut changed_index: bool = false;
        {
            let index = &borrow.index;
            let mut entries = index.each_mut_entry()?;
            while let Some(mut entry) = entries.pop() {
                let stat = self.stat.get(&entry.get_path());
                match stat {
                    Some(stat) => {
                        if !entry.is_stat_match(stat) {
                            self.changed.insert(entry.get_path());
                            continue;
                        }

                        // we want to avoid whatever is possible to read a file's content
                        if entry.times_match(stat) {
                            continue;
                        }

                        let mut blob = Blob::new(entry.path.to_path_buf())?;

                        // if the file has not changed despite the previous checks, it is necessary to
                        // update index info for the next time.
                        if entry.oid == blob.get_oid()? {
                            // println!("touched file: {:?}", entry);
                            entry.update_entry_stat(stat);
                            changed_index = true;
                        } else {
                            self.changed.insert(entry.get_path());
                        }
                    },
                    None => (),
                }
            };
        }
        if changed_index {
            let index = &mut borrow.index;
            index.update_changed_status();
        }
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

    pub fn status(self) -> Result<()> {
        let mut status = Status::new(self);
        status.run()
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
