use anyhow::Result;
use anyhow::anyhow;
use libflate::zlib::{Decoder, Encoder};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, Cursor, Read};
use std::path::Path;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use crate::{Blob, Commit, Object, Tree, util};

pub struct Database {
    pathname: PathBuf,
    objects: HashMap<String, ObjectType>
}

pub enum ObjectType {
    CommitType{ commit: Commit},
    BlobType{blob: Blob},
    TreeType{tree: Tree}
}

impl Database {
    pub fn new(path_buf: &PathBuf) -> Self {
        Database {
            pathname: path_buf.into(),
            objects: HashMap::new()
        }
    }

    // pub fn store(&self, object: &mut dyn Object) -> Result<()> {
    pub fn store<W: Object>(&self, object: &mut W) -> Result<()> {
        let data = object.get_data()?;
        self.write_object(&object.get_oid()?, data)
    }

    pub fn write_object(&self, oid: &Vec<u8>, content: Vec<u8>) -> Result<()> {
        let oid_s = util::encode_vec(&oid);
        let (a, b) = oid_s.split_at(2);
        let path = &self.pathname.join(a);
        if !path.exists() {
            fs::create_dir_all(&path).expect("unable to create path");
            let file_content = path.join(b);
            let mut file = OpenOptions::new()
                .read(true)
                .create(true)
                .write(true)
                .open(&file_content)?;

            let mut encoder = Encoder::new(Vec::new())?;
            io::copy(&mut &content[..], &mut encoder)?;
            let encode_data = encoder.finish().into_result()?;

            file.write_all(&encode_data)?
        }
        Ok(())
    }

    pub fn get_commit_tree(&mut self, head_oid: String) -> Result<String> {
        let commit = self.load(&head_oid)?;
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

    pub fn show_commit(&mut self, head_oid: String) -> Result<()> {
        let commit_tree_ref = self.get_commit_tree(head_oid)?;

        self.show_tree(&commit_tree_ref, Path::new("").to_path_buf())?;
        Ok(())
    }

    pub fn show_tree(&mut self, oid: &str, prefix: PathBuf) -> Result<()> {
        let mut work = vec![(oid.to_string(), prefix)];
        while let Some((oid_, prefix_)) = work.pop() {
            let tree = self.load(&oid_)?;
            match tree {
                ObjectType::CommitType{commit: _} => {
                },
                ObjectType::BlobType{blob: _} => {
                },
                ObjectType::TreeType{ref tree} => {
                    // println!("to process: {:?}", tree);
                    for e in tree.entries.iter() {
                        // println!("to process entry: {:?}, is tree {:?}", e, e.is_tree());
                        let path = prefix_.join(&e.name);
                        if e.is_tree() {
                            let oid_inner = util::encode_vec(&e.get_oid()?);
                            work.push((oid_inner, path))
                        } else {
                            let mode = &e.mode;
                            let oid_inner = util::encode_vec(&e.get_oid()?);
                            println!("{} {:?} {:?}", mode, oid_inner, path);
                        }
                    }
                },
            }
        }
        Ok(())
    }

    pub fn load(&mut self, oid: &str) -> Result<&ObjectType>  {
        if self.objects.contains_key(oid) {
            Ok(self.objects.get(oid).unwrap())
        } else {
            let key = oid.to_string();
            let e = self.read_object(&oid)?;
            self.objects.insert(key, e);
            Ok(self.objects.get(oid).unwrap())
        }
    }

    pub fn read_object(&self, oid: &str) -> Result<ObjectType> {
        let (dir, file) = oid.split_at(2);
        let mut path_to_file = self.pathname.to_path_buf();
        path_to_file.push(dir);
        path_to_file.push(file);
        let file = fs::read(path_to_file)?;
        let mut decoder = Decoder::new(&file[..])?;
        let mut decode_data = Vec::new();
        decoder.read_to_end(&mut decode_data)?;
        let mut cursor = Cursor::new(decode_data.clone());
        let mut type_object = vec![];
        cursor.read_until(0x20u8, &mut type_object)?;
        let type_object= String::from_utf8(type_object)?;
        // println!("type_object: {:?}", type_object.trim());
        let mut length = vec![];
        cursor.read_until(0x00u8, &mut length)?;
        let length = String::from_utf8(length)?;
        // println!("length: {:?}", length);
        match type_object.trim().as_ref() {
            "commit" => { 
                let commit = Commit::parse(&mut cursor, oid)?;
                Ok(ObjectType::CommitType{commit})
                
            } ,
            "blob" => { 
                let blob = Blob::parse(&mut cursor)?;
                Ok(ObjectType::BlobType{blob})

            },
            "tree" => { 
                let tree = Tree::parse(&mut cursor, oid.as_bytes().to_vec())?; 
                Ok(ObjectType::TreeType{tree})
            },
            _ => {
                println!("1: {}", type_object);
                return Err(anyhow!("unknow object type"))
            }
        }
    }
}
