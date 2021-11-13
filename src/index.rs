use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs::Metadata;
use std::fs::OpenOptions;
use std::{io::Write, os::unix::prelude::MetadataExt, path::PathBuf, u16, u8, usize};

use anyhow::Result;

use crate::util;

pub struct Index {
    pathname: PathBuf,
    entries: HashMap<String, Entry>,
    keys: BTreeSet<String>
}

#[derive(Clone)]
struct Entry {
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
    oid: String,
    flags: u16,
    path: String,
}

const ENTRY_BLOCK: usize = 8;
const REGULAR_MODE: u32 = 0o100644_u32;
const EXECUTABLE_MODE: u32 = 0o100755_u32;
const MAX_PATH_SIZE: u16 = 0xfff;

impl Entry {
    fn get_data(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let oid = hex::decode(&self.oid.clone())?;
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
        data.extend_from_slice(&oid);
        data.extend_from_slice(&self.flags.to_be_bytes());
        data.extend_from_slice(&self.path.as_bytes());
        data.push(0x00);
        while data.len() % ENTRY_BLOCK != 0 {
            data.push(0x00);
        }
        Ok(data)
    }

    fn create(pathname: PathBuf, oid: String, stat: Metadata) -> Result<Self> {
        let path = pathname.to_str().unwrap();
        let mode = if (stat.mode() & 0o001) != 0 {
            EXECUTABLE_MODE
        } else {
            REGULAR_MODE
        };
        let flags = std::cmp::min(path.bytes().len() as u16, MAX_PATH_SIZE);
        let entry = Entry {
            ctime: stat.ctime() as u32,
            ctime_nsec: stat.ctime_nsec() as u32,
            mtime: stat.mtime() as u32,
            mtime_nsec: stat.mtime_nsec() as u32,
            dev: stat.dev() as u32,
            ino: stat.ino() as u32,
            mode: mode,
            uid: stat.uid() as u32,
            gid: stat.gid() as u32,
            size: stat.size() as u32,
            path: path.to_owned(),
            oid,
            flags,
        };
        Ok(entry)
    }

    fn key(self) -> String {
        self.path
    }
}

impl Index {
    pub fn new(pathname: PathBuf) -> Self {
        Index {
            pathname,
            entries: HashMap::new(),
            keys: BTreeSet::new()
        }
    }

    pub fn add(&mut self, pathname: PathBuf, oid: String, stat: Metadata) -> Result<()> {
        // let path = pathname.to_str().unwrap();
        let entry = Entry::create(pathname.clone(), oid, stat)?;
        self.keys.insert(entry.clone().key());
        self.entries.insert(entry.clone().key(), entry);
        Ok(())
    }

    fn each_entry(&self) -> Result<Vec<&Entry>> {
        let mut entries: Vec<&Entry> = Vec::new();
        self.keys.iter().for_each(|k| entries.push(self.entries.get(k).unwrap()));
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
        let oid = util::hexdigest(&data);
        let mut data_to_write = data;
        let oid = hex::decode(oid)?;
        data_to_write.extend_from_slice(&oid);

        let mut file = OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(&self.pathname)?;

        file.write_all(&data_to_write)?;
        Ok(())
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
