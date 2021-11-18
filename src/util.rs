use anyhow::Result;
use data_encoding::HEXLOWER;
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, Metadata},
    os::unix::prelude::MetadataExt,
    path::{Path, PathBuf},
};

use crate::{Database, Entry, EntryAdd, Tree};

#[derive(Eq, Clone, PartialEq, PartialOrd, Debug)]
pub enum TreeEntry {
    TreeBranch { tree: Tree, name: String },
    TreeLeaf { entry: Entry, name: String },
}

#[derive(Debug)]
pub enum TreeEntryAux {
    TreeBranchAux { tree: TreeAux },
    TreeLeafAux { entry: Entry },
}

#[derive(Debug)]
pub struct TreeAux {
    pub entries: HashMap<PathBuf, TreeEntryAux>,
}
impl TreeAux {
    pub fn new() -> Self {
        TreeAux {
            entries: HashMap::new(),
        }
    }
    pub fn add_entry(&mut self, ancestors: Vec<PathBuf>, entry_add: EntryAdd) -> Result<()> {
        let tea = TreeEntryAux::TreeBranchAux { tree: TreeAux::new() };
        if !ancestors.is_empty() {
                let first = ancestors.first().unwrap();
                let mut comps = first.components();
                let comp = comps.next_back().unwrap();
                let comp: &Path = comp.as_ref();
                if !self.entries.contains_key(comp) {
                    self.entries.insert(comp.to_path_buf(), tea);
                } 
                let e: &mut TreeEntryAux = self.entries.get_mut(comp).unwrap();
                match e {
                    TreeEntryAux::TreeLeafAux { entry: _entry } => {}
                    TreeEntryAux::TreeBranchAux { tree } => {
                        if let Some((_, elements)) = ancestors.split_first() {
                            tree.add_entry(elements.to_vec(), entry_add)?;
                        }
                    }
                }
        } else {
                let mut comps = entry_add.path.components();
                let comp = comps.next_back().unwrap();
                let comp: &Path = comp.as_ref();
                let e = Entry::new(&entry_add.path, entry_add.oid)?;
                let leaf = TreeEntryAux::TreeLeafAux {
                    entry: e,
                };
                self.entries.insert(comp.to_path_buf(), leaf);
        }
        Ok(())
    }
}

impl Ord for TreeEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (
                TreeEntry::TreeBranch { tree: _, name: n1 },
                TreeEntry::TreeBranch { tree: _, name: n2 },
            ) => n1.cmp(&n2),
            (
                TreeEntry::TreeBranch { tree: _, name: n1 },
                TreeEntry::TreeLeaf { entry: _, name: n2 },
            ) => n1.cmp(&n2),
            (
                TreeEntry::TreeLeaf { entry: _, name: n1 },
                TreeEntry::TreeLeaf { entry: _, name: n2 },
            ) => n1.cmp(&n2),
            (
                TreeEntry::TreeLeaf { entry: _, name: n1 },
                TreeEntry::TreeBranch { tree: _, name: n2 },
            ) => n1.cmp(&n2),
        }
    }
}

impl Display for TreeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeEntry::TreeBranch { tree: _, name } => f.write_fmt(format_args!("{}", name)),
            TreeEntry::TreeLeaf { entry: _, name } => f.write_fmt(format_args!("{}", name)),
        }
    }
}

pub fn get_data(entries: &mut Vec<TreeEntry>) -> Result<Vec<u8>> {
    let mut acc_data: Vec<u8> = Vec::new();
    entries.sort_by(|a, b| match (a, b) {
        (
            TreeEntry::TreeBranch { tree: _, name: n1 },
            TreeEntry::TreeBranch { tree: _, name: n2 },
        ) => n1.cmp(&n2),
        (
            TreeEntry::TreeBranch { tree: _, name: n1 },
            TreeEntry::TreeLeaf { entry: _, name: n2 },
        ) => n1.cmp(&n2),
        (
            TreeEntry::TreeLeaf { entry: _, name: n1 },
            TreeEntry::TreeLeaf { entry: _, name: n2 },
        ) => n1.cmp(&n2),
        (
            TreeEntry::TreeLeaf { entry: _, name: n1 },
            TreeEntry::TreeBranch { tree: _, name: n2 },
        ) => n1.cmp(&n2),
    });
    entries
        .into_iter()
        .map(|entry| match entry {
            TreeEntry::TreeLeaf { entry, name: _ } => {
                acc_data.extend(entry.get_data()?);
                acc_data.to_vec();
                Ok(())
            }
            TreeEntry::TreeBranch { tree, name: _ } => {
                let mut data = Vec::new();
                data.extend_from_slice("040000".as_bytes());
                data.push(0x20u8);
                data.extend_from_slice(
                    tree.parent
                        .file_name()
                        .expect("unable to get filename")
                        .to_str()
                        .expect("invalid filename")
                        .as_bytes(),
                );
                data.push(0x00u8);
                data.extend_from_slice(&tree.oid);
                acc_data.extend(data);
                acc_data.to_vec();
                Ok(())
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(acc_data)
}
// let mut comps = e.path.components();
// let paths: Vec<_> =
//     comps.map(|e| e.as_os_str() ).map(|e| Path::new(e).to_path_buf()).collect();
pub fn build(entries_add: Vec<&EntryAdd>, db: &Database) -> Result<String> {
    let mut root = TreeAux::new();
    for e in entries_add.into_iter() {
        let mut ancestors: Vec<_> = 
            e.path.ancestors().filter(|en| en.to_path_buf() != e.path && en.exists()).map(|e| e.to_path_buf()).collect();
        ancestors.reverse();
        root.add_entry(ancestors, e.clone())?;
    }
    let mut trees = Vec::new();
    for (entry, aux) in root.entries {
        let t = Entry::build_entry(entry, aux, db)?;
        trees.push(t)
    }
    let entries_data = get_data(&mut trees)?;

    let length = entries_data.len();

    let mut data = Vec::new();

    data.extend_from_slice("tree".as_bytes());
    data.push(0x20u8);
    data.extend_from_slice(length.to_string().as_bytes());
    data.push(0x00u8);
    data.extend(entries_data);

    let data_to_write = data;

    let oid = hexdigest_vec(&data_to_write);

    db.write_object(&oid, data_to_write)?;

    Ok(encode_vec(&oid))
}

pub fn print_tree_aux(tree: TreeAux, main_key: PathBuf) -> () {
    println!(" -- sub-keys: {:?} of {:?} ", tree.entries.keys(), main_key);
    for (entry, aux) in tree.entries {
        match aux {
            TreeEntryAux::TreeLeafAux { entry } => {
                println!(" -- sub-value: {:?}", entry);
            }
            TreeEntryAux::TreeBranchAux { tree } => {
                print_tree_aux(tree, entry);
            }
        }
    }
}

pub fn print_tree(tree: Tree) -> () {
    for entry in tree.entries {
        println!("parent root: {:?}, oid: {:?}", tree.parent, tree.oid);
        match entry {
            TreeEntry::TreeLeaf { entry, name: _ } => {
                println!(" -- parent branch: {:?}", entry.path);
                println!(" ---- entry: {:?}, oid: {:?}", entry.name, entry.oid);
            }
            TreeEntry::TreeBranch { tree, name: _ } => {
                // println!("--parent branch: {:?}", b.parent);
                print_tree(tree)
            }
        }
    }
}

pub fn read_file(path: PathBuf) -> Result<Vec<u8>> {
    let res = fs::read(path)?;
    Ok(res)
}

pub fn stat_file(path: PathBuf) -> Result<Metadata> {
    let metadata = fs::metadata(path)?;
    Ok(metadata)
}

fn is_executable(entry: &PathBuf) -> Result<bool> {
    let metadata = fs::metadata(entry)?;
    let unix_mode = metadata.mode();
    Ok((unix_mode & 0o001) != 0)
}
pub fn get_mode(path_buf: PathBuf) -> Result<String> {
    let entry = &path_buf;
    if is_executable(entry)? {
        Ok("100755".to_string())
    } else {
        Ok("100644".to_string())
    }
}

pub fn encode_vec(data: &Vec<u8>) -> String {
    HEXLOWER.encode(data)
}

pub fn hexdigest_vec(data: &Vec<u8>) -> Vec<u8> {
    let mut context = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);

    context.update(&data);

    let digest = context.finish();

    digest.as_ref().to_vec()
}
