use anyhow::Result;

use crate::{util, Author, Object};

pub struct Commit<'a> {
    tree_ref: &'a str,
    author: Author,
    parent: Option<String>,
    message: String,
    type_: &'a str,
    oid: Option<Vec<u8>>,
}

impl Object for Commit<'_> {
    fn get_data(&self) -> Result<Vec<u8>> {
        self.get_data_to_write()
    }

    fn type_(&self) -> &str {
        &self.type_
    }

    fn get_oid(&mut self) -> Result<Vec<u8>> {
        match &self.oid {
            Some(oid) => Ok(oid.to_vec()),
            None => {
                let digest = util::hexdigest_vec(&self.get_data_to_write()?);
                self.set_oid(&digest);
                Ok(digest)
            }
        }
    }
}

impl Commit<'_> {
    pub fn new<'a>(tree_ref: &'a str, author: Author, message: String, parent: Option<String>) -> Commit<'a> {
        Commit {
            tree_ref,
            author,
            parent,
            message,
            type_: "commit",
            oid: None,
        }
    }

    fn get_data_to_write(&self) -> Result<Vec<u8>> {
        let mut lines = Vec::new();
        lines.push(format!("tree {}", &self.tree_ref));
        self.parent
            .as_ref()
            .map(|e| lines.push(format!("parent {}", e)));
        lines.push(format!("author {}", &self.author.to_s()));
        lines.push(format!("commiter {}", &self.author.to_s()));
        lines.push("".to_string());
        lines.push(self.message.clone());
        let data_to_write = lines.join("\n").as_bytes().to_vec();

        let mut data = Vec::new();

        let length = data_to_write.len();

        data.extend_from_slice("commit".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00);
        data.extend(&data_to_write);
        Ok(data)
    }

    fn set_oid(&mut self, oid: &Vec<u8>) -> () {
        self.oid = Some(oid.to_vec());
    }
}
