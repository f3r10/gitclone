use anyhow::Result;
use data_encoding::HEXLOWER;
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};
use std::{fmt::Display, fs, os::unix::prelude::MetadataExt, path::PathBuf};

use crate::{Entry, Tree};

#[derive(Eq, Clone, PartialEq, PartialOrd)]
pub enum TreeEntry {
    TreeBranch { tree: Tree, name: String },
    TreeLeaf { entry: Entry, name: String },
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

impl Display for TreeEntry{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeEntry::TreeBranch { tree: _, name } => f.write_fmt(format_args!("{}", name)),
            TreeEntry::TreeLeaf { entry: _, name } => f.write_fmt(format_args!("{}", name))
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
                let oid = hex::decode(tree.oid.clone())?;
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
                data.extend_from_slice(&oid);
                acc_data.extend(data);
                acc_data.to_vec();
                Ok(())
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(acc_data)
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

pub fn hexdigest(data: &Vec<u8>) -> String {
    let mut context = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);

    context.update(&data);

    let digest = context.finish();

    HEXLOWER.encode(digest.as_ref())
}
