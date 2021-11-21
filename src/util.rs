use anyhow::Result;
use anyhow::anyhow;
use data_encoding::HEXLOWER;
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};
use std::{
    collections::HashMap,
    env::current_dir,
    fs::{self, Metadata},
    io,
    os::unix::prelude::MetadataExt,
    path::{Path, PathBuf},
};

use crate::Object;
use crate::{Entry, EntryWrapper};

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
    pub fn add_entry(
        &mut self,
        ancestors: Vec<PathBuf>,
        root_path: PathBuf,
        oid_o: Option<Vec<u8>>,
    ) -> Result<()> {
        let tea = TreeEntryAux::TreeBranchAux {
            tree: TreeAux::new(),
        };
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
                        tree.add_entry(elements.to_vec(), root_path.to_path_buf(), oid_o)?;
                    }
                }
            }
        } else {
            let mut comps = root_path.components();
            let comp = comps.next_back().unwrap();
            let comp: &Path = comp.as_ref();
            let e = Entry::new(&root_path, oid_o)?;
            let leaf = TreeEntryAux::TreeLeafAux { entry: e };
            self.entries.insert(comp.to_path_buf(), leaf);
        }
        Ok(())
    }
}

pub fn get_data(entries: &mut Vec<EntryWrapper>) -> Result<Vec<u8>> {
    let mut acc_data: Vec<u8> = Vec::new();
    entries.sort_by(|a, b| match (a, b) {
        (
            EntryWrapper::EntryTree { tree: _, name: n1 },
            EntryWrapper::EntryTree { tree: _, name: n2 },
        ) => n1.cmp(&n2),
        (
            EntryWrapper::EntryTree { tree: _, name: n1 },
            EntryWrapper::Entry { entry: _, name: n2 },
        ) => n1.cmp(&n2),
        (
            EntryWrapper::Entry { entry: _, name: n1 },
            EntryWrapper::Entry { entry: _, name: n2 },
        ) => n1.cmp(&n2),
        (
            EntryWrapper::Entry { entry: _, name: n1 },
            EntryWrapper::EntryTree { tree: _, name: n2 },
        ) => n1.cmp(&n2),
    });
    entries
        .into_iter()
        .map(|entry| match entry {
            EntryWrapper::Entry { entry, name: _ } => {
                acc_data.extend(entry.get_data()?);
                acc_data.to_vec();
                Ok(())
            }
            EntryWrapper::EntryTree { tree, name: _ } => {
                let oid = tree.get_oid()?;
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
                data.extend_from_slice(&oid);
                acc_data.extend(data);
                acc_data.to_vec();
                Ok(())
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(acc_data)
}

pub fn read_file(path: PathBuf) -> Result<Vec<u8>> {
    let msg = format!("open ('{:?}'): Permission denied", &path);
    let res = fs::read(path).map_err(|_| anyhow!(msg))?;
    Ok(res)
}

pub fn stat_file(path: PathBuf) -> Result<Metadata> {
    let msg = format!("stat ('{:?}'): Permission denied", &path);
    let metadata = fs::metadata(path).map_err(|_| anyhow!(msg))?;
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

pub fn flatten_dot(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut e = paths
        .iter()
        .flat_map(|e| {
            if e.to_str().eq(&Some(".")) {
                let current_dir = current_dir()?;
                fs::read_dir(current_dir.clone())?
                    .filter(|e| match e {
                        Ok(p) => p.file_name() != ".git",
                        Err(_e) => true,
                    })
                    // since all the path are taken from the current dir the strip_prefix will not
                    // fail
                    .map(|res| {
                        res.map(|e| e.path().strip_prefix(&current_dir).unwrap().to_path_buf())
                    })
                    .collect::<Result<Vec<_>, io::Error>>()
            } else {
                Ok(vec![e.to_path_buf()])
            }
        })
        .flatten()
        .collect::<Vec<PathBuf>>();
    e.sort_by(|p1, p2| p1.cmp(p2));
    e.dedup();
    Ok(e)
}
