use anyhow::anyhow;
use byteorder::{BigEndian, ReadBytesExt};
use std::char;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs::File;
use std::fs::Metadata;
use std::fs::OpenOptions;
use std::path::Path;
use std::{io::Write, os::unix::prelude::MetadataExt, path::PathBuf, u16, u8, usize};

use anyhow::Result;

use crate::util;
use crate::Checksum;

pub struct Index {
    pathname: PathBuf,
    entries: HashMap<String, EntryAdd>,
    keys: BTreeSet<String>,
    changed: bool,
}

#[derive(Clone, Debug)]
pub struct EntryAdd {
    ctime: u32,
    ctime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
    pub oid: Vec<u8>,
    flags: u16,
    pub path: PathBuf,
}

const ENTRY_BLOCK: usize = 8;
const MAX_PATH_SIZE: u16 = 0xfff;

impl EntryAdd {
    pub fn get_name(&self) -> String {
        let path = self.path.to_path_buf();
        path.file_name().unwrap().to_str().unwrap().to_string()
    }

    pub fn get_path(&self) -> String {
        let path = self.path.to_path_buf();
        path.to_str().unwrap().to_string()
    }
    pub fn get_mode(&self) -> Result<u32> {
        let mode = self.mode;
        Ok(mode)
    }
    pub fn get_data_to_tree(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mode = util::get_mode(self.path.to_path_buf())?;

        data.extend_from_slice(&mode.to_be_bytes());
        data.push(0x20u8);
        data.extend_from_slice(
            &self
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
                .as_bytes(),
        );
        data.push(0x00u8);
        data.extend_from_slice(&self.oid);
        Ok(data)
    }

    fn get_data(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.ctime.to_be_bytes());
        data.extend_from_slice(&self.ctime_nsec.to_be_bytes());
        data.extend_from_slice(&self.mtime.to_be_bytes());
        data.extend_from_slice(&self.mtime_nsec.to_be_bytes());
        data.extend_from_slice(&self.dev.to_be_bytes());
        data.extend_from_slice(&self.ino.to_be_bytes());
        data.extend_from_slice(&self.mode.to_be_bytes());
        data.extend_from_slice(&self.uid.to_be_bytes());
        data.extend_from_slice(&self.gid.to_be_bytes());
        data.extend_from_slice(&self.size.to_be_bytes());
        data.extend_from_slice(&self.oid);
        data.extend_from_slice(&self.flags.to_be_bytes());
        data.extend_from_slice(&self.path.to_str().unwrap().to_string().as_bytes());
        data.push(0x00);
        while data.len() % ENTRY_BLOCK != 0 {
            data.push(0x00);
        }
        Ok(data)
    }

    fn create(pathname: PathBuf, oid: Vec<u8>, stat: Metadata) -> Result<Self> {
        let path = pathname.to_str().unwrap();
        let mode =  util::get_mode(pathname.to_path_buf())?;
        let flags = std::cmp::min(path.bytes().len() as u16, MAX_PATH_SIZE);
        let entry = EntryAdd {
            ctime: stat.ctime() as u32,
            ctime_nsec: stat.ctime_nsec() as u32,
            mtime: stat.mtime() as u32,
            mtime_nsec: stat.mtime_nsec() as u32,
            dev: stat.dev() as u32,
            ino: stat.ino() as u32,
            mode,
            uid: stat.uid() as u32,
            gid: stat.gid() as u32,
            size: stat.size() as u32,
            path: pathname,
            oid,
            flags,
        };
        Ok(entry)
    }

    pub fn key(&self) -> String {
        self.path.to_str().unwrap().to_string()
    }

    fn parse(entry: Vec<u8>) -> Result<EntryAdd> {
        let mut stats = Vec::new();
        let (numbers_vec, tail) = entry.split_at(40);
        let (oid, tail) = tail.split_at(20);
        let oid = oid.to_vec();
        let (mut flag_vec, path_vec) = tail.split_at(2);
        let path = String::from_utf8(path_vec.to_vec())?
            .trim_matches(char::from(0))
            .to_string();
        let path = Path::new(&path).to_path_buf();
        let flags = flag_vec.read_u16::<BigEndian>()?;
        for mut chunk in numbers_vec.chunks_exact(4) {
            stats.push(chunk.read_u32::<BigEndian>()?)
        }
        let e = EntryAdd {
            ctime: stats[0],
            ctime_nsec: stats[1],
            mtime: stats[2],
            mtime_nsec: stats[3],
            dev: stats[4],
            ino: stats[5],
            mode: stats[6],
            uid: stats[7],
            gid: stats[8],
            size: stats[9],
            path,
            oid,
            flags,
        };
        Ok(e)
    }
}

const HEADER_SIZE: usize = 12;
const SIGNATURE: &str = "DIRC";
const VERSION: u32 = 2;
const ENTRY_MIN_SIZE: usize = 64;
impl Index {
    pub fn new(pathname: &PathBuf) -> Self {
        Index {
            pathname: pathname.to_path_buf(),
            entries: HashMap::new(),
            keys: BTreeSet::new(),
            changed: false,
        }
    }

    fn clear(&mut self) -> Result<()> {
        self.entries = HashMap::new();
        self.keys = BTreeSet::new();
        self.changed = false;
        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        self.clear()?;
        let mut reader = Checksum::new(File::open(&self.pathname)?);
        let count = &self.read_header(&mut reader)?;
        self.read_entries(&mut reader, *count)?;
        reader.verify_checksum()?;
        Ok(())
    }

    pub fn read_entries(&mut self, reader: &mut Checksum, count: u32) -> Result<()> {
        for _ in 0..count {
            let mut entry = reader.read(ENTRY_MIN_SIZE, true)?;
            while *entry.last().unwrap() != 0u8 {
                entry.extend_from_slice(&reader.read(ENTRY_BLOCK, true)?)
            }
            self.store_entry(EntryAdd::parse(entry)?)?;
        }

        Ok(())
    }

    pub fn read_header(&self, reader: &mut Checksum) -> Result<u32> {
        let data = reader.read(HEADER_SIZE, true)?;
        let mut chunks: Vec<_> = data.chunks_exact(4).collect();
        let signature = String::from_utf8(chunks[0].to_vec())?;
        let version = chunks[1].read_u32::<BigEndian>()?;
        let count = chunks[2].read_u32::<BigEndian>()?;
        //NOTE The cast from slice to array is more difficult that it looks like.
        // https://stackoverflow.com/questions/25428920/how-to-get-a-slice-as-an-array-in-rust
        // println!("data2: {:?}", u32::from_be_bytes(t1[..]));
        // println!("data2: {:?}", BigEndian::read_u32(&chunks[1][..]));
        if signature != SIGNATURE {
            return Err(anyhow!(format!(
                "Signature: expected: {} but found {}",
                SIGNATURE, signature
            )));
        }
        if version != VERSION {
            return Err(anyhow!(format!(
                "Version: expected: {} but found {}",
                VERSION, version
            )));
        }
        Ok(count)
    }

    fn store_entry(&mut self, entry: EntryAdd) -> Result<()> {
        //TODO find a better way that cloning the entry
        self.keys.insert(entry.clone().key());
        self.entries.insert(entry.clone().key(), entry);
        Ok(())
    }

    pub fn add(&mut self, pathname: PathBuf, oid: Vec<u8>, stat: Metadata) -> Result<()> {
        // let path = pathname.to_str().unwrap();
        let entry = EntryAdd::create(pathname.clone(), oid, stat)?;
        self.discard_conflicts(&entry)?;
        self.store_entry(entry)?;
        self.changed = true;
        Ok(())
    }

    pub fn discard_conflicts(&mut self, entry: &EntryAdd) -> Result<()> {
        for parent in entry.path.ancestors() {
            let parent_str = parent.to_str().ok_or(anyhow!("unable to get parent filename"))?;
            self.keys.remove(parent_str);
            self.entries.remove(parent_str);
        }
        let entry_name = entry.path.to_str().ok_or(anyhow!("unable to get filename"))?;

        // TODO these two methods iterate over all the elements which can be really expensive
        // depending on how many keys the collections have.
        // The alternative would be to create another HashMap where it should be stored:
        // {Parent -> [Child]}, so if it would be a conflicting file name with a dir name, it
        // would be necessary to get the corresponding [child] list to remove from the self.entries.
        self.entries.retain(|k, _| !k.contains(entry_name));
        self.keys.retain(|k| !k.contains(entry_name));
        Ok(())
    }

    pub fn each_entry(&self) -> Result<Vec<&EntryAdd>> {
        let mut entries: Vec<&EntryAdd> = Vec::new();
        self.keys
            .iter()
            .for_each(|k| entries.push(self.entries.get(k).unwrap()));
        Ok(entries)
    }

    pub fn write_updates(&self) -> Result<()> {
        let mut data = Vec::new();
        let mode: i32 = 2;
        let len: i32 = *&self.entries.len() as i32;
        data.extend_from_slice("DIRC".as_bytes());
        data.extend_from_slice(&mode.to_be_bytes());
        data.extend_from_slice(&len.to_be_bytes());
        let entries = self.each_entry()?;
        for v in entries {
            data.extend_from_slice(&v.get_data().unwrap());
        }
        let oid = util::hexdigest_vec(&data);
        let mut data_to_write = data;
        data_to_write.extend_from_slice(&oid);

        let mut file = OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(&self.pathname)?;

        file.write_all(&data_to_write)?;
        Ok(())
    }

    pub fn is_tracked(&self, path: PathBuf) -> bool {
        // this checks for filename or dirs
        // self.entries.contains_key(&path.to_str().unwrap().to_string())
        // TODO this is not much performance if the list of elements of the workspace is huge. It
        // is neccesary to create an aux data structure to save parent directories.  
        let entry_name = path.to_str().unwrap();
        self.entries.iter().any(|(key, _)| key.contains(entry_name))
    }
}

// 00000000  44 49 52 43 00 00 00 02  00 00 00 01 61 83 28 ca  |DIRC........a.(.|
// 00000010  33 94 c6 d5 61 83 28 c1  05 82 98 89 00 00 08 03  |3...a.(.........|
// 00000020  00 12 5c 6d 00 00 81 a4  00 00 03 e8 00 00 03 e8  |..\m............|
// 00000030  00 00 00 00 e6 9d e2 9b  b2 d1 d6 43 4b 8b 29 ae  |...........CK.).|
// 00000040  77 5a d8 c2 e4 8c 53 91  00 0e 74 32 2f 74 32 5f  |wZ....S...t2/t2_|
// 00000050  74 65 78 74 2e 74 78 74  00 00 00 00 b1 9c f0 a0  |text.txt........|
// 00000060  b9 90 50 8f c2 26 52 48  dc 4f f7 06 71 ae bd 1d  |..P..&RH.O..q...|
// 00000070
//
//
//
// 00000000  44 49 52 43 00 00 00 02  00 00 00 01 61 83 28 ca  |DIRC........a.(.|
// 00000010  33 94 c6 d5 61 83 28 c1  05 82 98 89 00 00 08 03  |3...a.(.........|
// 00000020  00 12 5c 6d *00 01 89 24*  00 00 03 e8 00 00 03 e8  |..\m...$........|
// 00000030  00 00 00 00 e6 9d e2 9b  b2 d1 d6 43 4b 8b 29 ae  |...........CK.).|
// 00000040  77 5a d8 c2 e4 8c 53 91  00 0e 74 32 2f 74 32 5f  |wZ....S...t2/t2_|
// 00000050  74 65 78 74 2e 74 78 74  00 00 00 00 6e 40 74 f9  |text.txt....n@t.|
// 00000060  2c 1d 98 47 68 83 4d a3  e1 49 d2 31 85 37 84 ad  |,..Gh.M..I.1.7..|
// 00000070
//
// The solution was to add a `o` on the mode representation
// instead of 0100644 to 0o100644 becuase in this way rust understand octal representation
// 00000000  44 49 52 43 00 00 00 02  00 00 00 01 61 83 28 ca  |DIRC........a.(.|
// 00000010  33 94 c6 d5 61 83 28 c1  05 82 98 89 00 00 08 03  |3...a.(.........|
// 00000020  00 12 5c 6d 00 00 81 a4  00 00 03 e8 00 00 03 e8  |..\m............|
// 00000030  00 00 00 00 e6 9d e2 9b  b2 d1 d6 43 4b 8b 29 ae  |...........CK.).|
// 00000040  77 5a d8 c2 e4 8c 53 91  00 0e 74 32 2f 74 32 5f  |wZ....S...t2/t2_|
// 00000050  74 65 78 74 2e 74 78 74  00 00 00 00 b1 9c f0 a0  |text.txt........|
// 00000060  b9 90 50 8f c2 26 52 48  dc 4f f7 06 71 ae bd 1d  |..P..&RH.O..q...|
// 00000070
