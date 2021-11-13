use crate::{Author, Object, util};

pub struct Commit {
    tree_ref: String,
    author: Author,
    parent: Option<String>,
    message: String,
    type_: String,
    pub oid: String,
    data_to_write: Vec<u8>
}

impl Object for Commit {
    fn get_data(&self) -> Vec<u8> {
        self.data_to_write.clone()
    }

    fn type_(&self) -> &str {
        &self.type_
    }

    fn get_oid(&self) -> &str {
        &self.oid
    }
}

impl Commit {
    pub fn new(tree_ref: String, author: Author, message: String, parent: Option<String>) -> Self {
        let mut lines = Vec::new();
        lines.push(format!("tree {}", tree_ref));
        parent.as_ref().map(|e| lines.push(format!("parent {}", e)));
        lines.push(format!("author {}", author.to_s()));
        lines.push(format!("commiter {}", author.to_s()));
        lines.push("".to_string());
        lines.push(message.clone());
        let data_to_write = lines.join("\n").as_bytes().to_vec();

        let mut data = Vec::new();

        let length = data_to_write.len();

        data.extend_from_slice("commit".as_bytes());
        data.push(0x20u8);
        data.extend_from_slice(length.to_string().as_bytes());
        data.push(0x00);
        data.extend(&data_to_write);
        let digest = util::hexdigest(&data);
        Commit {
            tree_ref,
            author,
            parent,
            message,
            type_: "commit".to_string(),
            oid: digest,
            data_to_write: data
        }
    }
}
