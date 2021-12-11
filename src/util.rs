use anyhow::Result;
use anyhow::anyhow;
use data_encoding::HEXLOWER;
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};
use std::fs::File;
use std::io::Write;
use std::os::unix::prelude::PermissionsExt;
use std::{
    collections::HashMap,
    env::current_dir,
    fs::{self, Metadata},
    path::{Path, PathBuf},
};

use crate::Entry;

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
        mode: u32
    ) -> Result<()> {
        let tea = TreeEntryAux::TreeBranchAux {
            tree: TreeAux::new(),
        };
        if !ancestors.is_empty() {
            let first = ancestors.first().ok_or(anyhow!("unable to get first ancestor"))?;
            let mut comps = first.components();
            let comp = comps.next_back().ok_or(anyhow!("unable to get last component of path"))?;
            let comp: &Path = comp.as_ref();
            if !self.entries.contains_key(comp) {
                self.entries.insert(comp.to_path_buf(), tea);
            }
            let e: &mut TreeEntryAux = self.entries.get_mut(comp).ok_or(anyhow!("unable to get mut component from HasMap"))?;
            match e {
                TreeEntryAux::TreeLeafAux { entry: _entry } => {}
                TreeEntryAux::TreeBranchAux { tree } => {
                    if let Some((_, elements)) = ancestors.split_first() {
                        tree.add_entry(elements.to_vec(), root_path.to_path_buf(), oid_o, mode)?;
                    }
                }
            }
        } else {
            let mut comps = root_path.components();
            let comp = comps.next_back().ok_or(anyhow!("unable to get last component of path"))?;
            let comp: &Path = comp.as_ref();
            let mode = get_mode_u(mode);
            let name = root_path.file_name().expect("Expected a name").to_str().expect("Invalif filename").to_string();
            let e = Entry::new(mode, oid_o.unwrap(), root_path.to_path_buf(), name, vec![]);
            let leaf = TreeEntryAux::TreeLeafAux { entry: e };
            self.entries.insert(comp.to_path_buf(), leaf);
        }
        Ok(())
    }
}

pub fn read_file(path: PathBuf) -> Result<Vec<u8>> {
    let msg = format!("open ('{:?}'): Permission denied", &path);
    let res = fs::read(path).map_err(|_| anyhow!(msg))?;
    Ok(res)
}

pub fn stat_file(path: &PathBuf) -> Result<Metadata> {
    let msg = format!("stat ('{:?}'): Permission denied", &path);
    let metadata = fs::metadata(path).map_err(|_| anyhow!(msg))?;
    Ok(metadata)
}

pub fn is_executable(entry: &PathBuf) -> Result<bool> {
    let metadata = fs::metadata(entry)?;
    let unix_mode = metadata.permissions().mode();
    Ok((unix_mode & 0o001) != 0)
}

const REGULAR_MODE: u32 = 0o100644;
const EXECUTABLE_MODE: u32 = 0o100755;
pub fn get_mode(path_buf: PathBuf) -> Result<u32> {
    let entry = &path_buf;
    if is_executable(entry)? {
        Ok(EXECUTABLE_MODE)
    } else {
        Ok(REGULAR_MODE)
    }
}

pub fn get_mode_u(mode_u: u32) -> String {
    let mut mode = String::new();
    let is_executable = (mode_u & 0o001) != 0;
    if is_executable {
        mode.push_str("100755")
    } else {
        mode.push_str("100644")
    }
    mode
}

pub fn get_mode_stat(stat: &Metadata) -> u32 {
    let unix_mode = stat.permissions().mode();
    let is_executable = (unix_mode & 0o001) != 0;
    if is_executable {
        EXECUTABLE_MODE
    } else {
        REGULAR_MODE
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
                    .map(|res| {
                        let e = res?;
                        let path = e.path();
                        let stripped = path.strip_prefix(&current_dir)?;
                        Ok(stripped.to_path_buf())
                    })
                    .collect::<Result<Vec<_>, anyhow::Error>>()
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


pub fn list_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut res = Vec::new();
    let mut work = vec![path.to_path_buf()];
    while let Some(dir) = work.pop() {
        let filtered = fs::read_dir(dir)?
            .into_iter()
            .filter(|e| match e {
                Ok(p) => p.file_name() != ".git" && p.file_name() != "target",
                Err(_e) => true,
            });
        for entry in filtered {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_file() {
                res.push(entry.path())
            } else if file_type.is_dir() {
                work.push(entry.path())
            }
        }
    }
    Ok(res)
}

pub fn write_file (root_path: &PathBuf, paths: Vec<(PathBuf, &[u8])>) -> Result<Vec<PathBuf>> {
    let mut final_paths = Vec::new();
    for (p, buf) in paths.iter() {
        match p.parent() {
            Some(parent) if parent.file_name().is_some() => {
                let path = root_path.join(parent);
                fs::create_dir_all(&path)?;
                File::create(&(root_path.join(p)))?.write_all(buf)?;
                final_paths.push(Path::new(p).to_path_buf());
            },
            None | Some(_) => {
                let path = root_path.join(p);
                File::create(&path)?.write_all(buf)?;
                final_paths.push(Path::new(p).to_path_buf());
            },
        }
    }
    Ok(final_paths)
}
