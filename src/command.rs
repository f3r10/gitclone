use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{DirEntry, Metadata};
use std::path::Path;
use std::rc::Rc;
use std::{fs, path::PathBuf};

use anyhow::anyhow;
use anyhow::Result;
use chrono::Local;

use crate::database::ObjectType;
use crate::tree::{self, Tree};
use crate::{Blob, Entry, util};
use crate::Author;
use crate::Commit;
use crate::Object;
use crate::Refs;
use crate::{Database, Index, Workspace};

pub struct Command {
    workspace: Workspace,
    db: Database,
    pub index: Index,
    refs: Refs
}


#[derive(Ord, PartialEq, PartialOrd, Eq)]
pub enum WorkspaceStatus{
    Deleted,
    Modified,
    Default
}
#[derive(Ord, PartialEq, PartialOrd, Eq)]
pub enum IndexStatus{
    Added,
    Modified,
    Deleted,
    Default
}

pub struct Status {
    stat: HashMap<String, Metadata>,
    untracked: BTreeSet<String>,
    changed: BTreeMap<String, (WorkspaceStatus, IndexStatus)>,
    cmd: Rc<RefCell<Command>>,
    head_tree: HashMap<String, Entry>
}

impl Status {
    pub fn new(cmd: Command) -> Self {
        Status {
            stat: HashMap::new(),
            untracked: BTreeSet::new(),
            changed: BTreeMap::new(),
            cmd: Rc::new(RefCell::new(cmd)),
            head_tree: HashMap::new()
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
        if cmd.borrow().refs.head_path().exists() {
            self.load_head_tree()?;
        }
        self.check_index_entries()?;
        self.collect_deleted_head_entries()?;
        cmd.borrow().index.write_updates()?;
        self.changed.iter().for_each(|(path, status)| {
            match status {
                (WorkspaceStatus::Deleted, IndexStatus::Added) => {
                    println!(" AD {}", path)
                },
                (WorkspaceStatus::Deleted, IndexStatus::Default) => {
                    println!(" D {}", path)
                },
                (WorkspaceStatus::Deleted, IndexStatus::Modified) => {
                    println!(" MD {}", path)
                },
                (WorkspaceStatus::Deleted, IndexStatus::Deleted) => {
                    println!(" DD {}", path)
                },
                (WorkspaceStatus::Modified, IndexStatus::Added) => {
                    println!(" AM {}", path)
                },
                (WorkspaceStatus::Modified, IndexStatus::Default) => {
                    println!(" M {}", path)
                },
                (WorkspaceStatus::Modified, IndexStatus::Modified) => {
                    println!(" MM {}", path)
                },
                (WorkspaceStatus::Modified, IndexStatus::Deleted) => {
                    println!(" DM {}", path)
                },
                (WorkspaceStatus::Default, IndexStatus::Default) => {
                },
                (WorkspaceStatus::Default, IndexStatus::Added) => {
                    println!(" A {}", path)
                },
                (WorkspaceStatus::Default, IndexStatus::Modified) => {
                    println!(" M {}", path)
                },
                (WorkspaceStatus::Default, IndexStatus::Deleted) => {
                    println!(" D {}", path)
                },
            }
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

    pub fn check_index_entries(&mut self) -> Result<()> {
        let cmd = self.cmd.clone();
        let borrow = &mut *cmd.borrow_mut();
        let mut changed_index: bool = false;
        {
            let index = &borrow.index;
            let mut entries = index.each_mut_entry()?;
            while let Some(mut entry) = entries.pop() {
                let mut head_tree_status = IndexStatus::Default; 
                if self.head_tree.contains_key(&entry.get_path()) {
                    let head_entry = self.head_tree.get(&entry.get_path()).unwrap();
                    if !(util::get_mode_u(entry.get_mode()?) == head_entry.mode && entry.oid == head_entry.sha1_hash) {
                        head_tree_status = IndexStatus::Modified
                    }
                } else {
                    head_tree_status = IndexStatus::Added
                }
                let stat = self.stat.get(&entry.get_path());
                match stat {
                    Some(stat) => {
                        if !entry.is_stat_match(stat) {
                            let workspace_status = WorkspaceStatus::Modified;
                            self.changed.insert(
                                entry.get_path(), 
                                (workspace_status, head_tree_status));
                            continue;
                        } 

                        // we want to avoid whatever is possible to read a file's content
                        if entry.times_match(stat) {
                            let workspace_status = WorkspaceStatus::Default;
                            self.changed.insert(
                                entry.get_path(), 
                                (workspace_status, head_tree_status));
                            continue;
                        }

                        let data = util::read_file(entry.path.to_path_buf())?;

                        let blob = Blob::new(data)?;

                        // if the file has not changed despite the previous checks, it is necessary to
                        // update index info for the next time.
                        if entry.oid == blob.get_oid()? {
                            // println!("touched file: {:?}", entry);
                            entry.update_entry_stat(stat);
                            changed_index = true;
                        } else {
                            let workspace_status = WorkspaceStatus::Modified;
                            self.changed.insert(
                                entry.get_path(), 
                                (workspace_status, head_tree_status));
                        }
                        }
                    None => { 
                        let workspace_status = WorkspaceStatus::Deleted;
                        self.changed.insert(
                            entry.get_path(), 
                            (workspace_status, head_tree_status));
                    },
                }
            };
        }
        if changed_index {
            let index = &mut borrow.index;
            index.update_changed_status();
        }
        Ok(())
    }

    pub fn load_head_tree(&mut self) -> Result<()> {
        let cmd = self.cmd.clone();
        let head_oid = cmd.borrow().refs.read_head().ok_or(anyhow!("unable to read HEAD file"))?;
        let commit = self.get_commit_tree(head_oid)?;
        self.read_tree(&commit, Path::new("").to_path_buf())?;
        Ok(())
    }


    fn get_commit_tree(&self, head_oid: String) -> Result<String> {
        let cmd = self.cmd.clone();
        let borrow = &mut *cmd.borrow_mut();
        let commit = borrow.db.load(&head_oid)?;
        match commit  {
            ObjectType::CommitType{commit: c} => {
                Ok(c.tree_ref.clone())
            },
            ObjectType::BlobType{blob: _} => {
                Err(anyhow!("this is not a valid commit object"))
            },
            ObjectType::TreeType{tree: _} => {
                Err(anyhow!("this is not a valid commit object"))
            },
        }
    }

    pub fn read_tree(&mut self, oid: &str, prefix: PathBuf) -> Result<()> {
        let cmd = self.cmd.clone();
        let borrow = &mut *cmd.borrow_mut();
        let mut work = vec![(oid.to_string(), prefix)];
        while let Some((oid_, prefix_)) = work.pop() {
            let tree = borrow.db.load(&oid_)?;
            match tree {
                ObjectType::CommitType{commit: _} => {
                },
                ObjectType::BlobType{blob: _} => {
                },
                ObjectType::TreeType{tree} => {
                    // println!("to process: {:?}", tree);
                    for e in tree.entries.iter() {
                        // println!("to process entry: {:?}, is tree {:?}", e, e.is_tree());
                        let path = prefix_.join(&e.name);
                        if e.is_tree() {
                            let oid_inner = util::encode_vec(&e.get_oid()?);
                            work.push((oid_inner, path))
                        } else {
                            self.head_tree.insert(path.display().to_string(), e.clone());
                            // let mode = &e.mode;
                            // let oid_inner = util::encode_vec(&e.get_oid()?);
                            // println!("{} {:?} {:?}", mode, oid_inner, path);
                        }
                    }
                },
            }
        }
        Ok(())
    }

    fn collect_deleted_head_entries(&mut self) -> Result<()> {
        for (path, _) in self.head_tree.iter() {
            if !self.cmd.borrow().index.is_tracked_file(path) {
                self.changed.insert(
                    path.to_string(), 
                    (WorkspaceStatus::Default, IndexStatus::Deleted));
            }
        }
        Ok(())
    }
}

impl Command {
    pub fn new(path_buf: PathBuf) -> Result<Self> {
        let ws = Workspace::new(&path_buf);
        let db = Database::new(&path_buf.join(".git/objects"));
        let index = Index::new(&path_buf.join(".git/index"));
        let refs = Refs::new(&ws.get_git_path());
        Ok(Command {
            workspace: ws,
            db,
            index,
            refs
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
        let tree = Tree::new_from_files(paths, &self.db)?;
        self.workspace
            .create_index_entry(&tree, &mut self.index)?;
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
        let mut tree = tree::build_add_tree(root)?;
        tree.save_tree(&self.db)?;
        let refs = Refs::new(&self.workspace.get_git_path());
        let parent = refs.read_head();
        let current_time = Local::now();
        let author = Author::new(author, email, current_time);
        let oid  = util::encode_vec(&tree.sha1_hash);
        let mut commit = Commit::new(
            oid,
            author,
            message.to_string(),
            parent.clone(),
            None
        )?;
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
}
