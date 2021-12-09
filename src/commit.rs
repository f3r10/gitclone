use std::{collections::HashMap, io::{BufRead, Cursor, Read}};

use anyhow::Result;

use crate::{util, Author, Object};

#[derive(Debug)]
pub struct Commit {
    pub tree_ref: String,
    author: Author,
    parent: Option<String>,
    message: String,
    type_: String,
    oid: Vec<u8>,
}

impl Object for Commit {
    fn get_data(&self) -> Result<Vec<u8>> {
        self.get_data_to_write()
    }

    fn type_(&self) -> &str {
        &self.type_
    }

    fn get_oid(&self) -> Result<Vec<u8>> {
        Ok(self.oid.to_vec())
    }
}

impl Commit {
    pub fn new(
        tree_ref: String, 
        author: Author, 
        message: String, 
        parent: Option<String>, 
        oid: Option<Vec<u8>>
        ) -> Result<Commit> {
        let digest = oid.unwrap_or({
            let data_to_write = get_data_to_write(tree_ref.as_str(), &author, message.as_str(), &parent)?;
            util::hexdigest_vec(&data_to_write)
        });
        Ok(Commit {
            tree_ref,
            author,
            parent,
            message,
            type_: "commit".to_string(),
            oid: digest,
        })
    }


    pub fn parse(cursor: &mut Cursor<Vec<u8>>, oid: &str) -> Result<Self> {
        let mut headers: HashMap<String, String> = HashMap::new();
        loop {
            let mut line = vec![];
            let _num_read = cursor.read_until(b'\n', &mut line)?;

            // if line.len() == 1 {
            //     for l in &line {
            //         println!("line: {:X}", l)
            //     }
            //     println!("new_line: {:?}", "\n".as_bytes().to_owned()[0]);
            //     println!("equal {:?}", line == vec![0b1010])
            // }
            if line == vec![0b1010] {
                break;
            }
            let elements: Vec<_> = line.strip_suffix("\n".as_bytes()).unwrap().splitn(2, |i| *i == 32).collect();
            let key = String::from_utf8(elements[0].to_vec())?;
            let value = String::from_utf8(elements[1].to_vec())?;
            headers.insert(key,value);
        }
        let mut message = vec![];
        cursor.read_to_end(&mut message)?;
        let message = String::from_utf8(message)?;
        let author = Author::parse(headers.get("author").unwrap())?;
        let tree = headers.remove("tree").unwrap();
        let commit = Commit::new(tree, author, message, headers.get("parent").map(|e| e.to_string()), Some(oid.as_bytes().to_owned()))?;
        Ok(commit)
    }

    fn get_data_to_write(&self) -> Result<Vec<u8>> {
        let data = get_data_to_write(&self.tree_ref, &self.author, self.message.as_str(), &self.parent)?;
        Ok(data)
    }
}

fn get_data_to_write(tree_ref: &str, author: &Author, message: &str, parent: &Option<String>) -> Result<Vec<u8>> {
    let mut lines = Vec::new();
    lines.push(format!("tree {}", tree_ref));
    parent
        .as_ref()
        .map(|e| lines.push(format!("parent {}", e)));
    lines.push(format!("author {}", author.to_s()));
    lines.push(format!("commiter {}", author.to_s()));
    lines.push("".to_string());
    lines.push(message.to_string());
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
