use anyhow::Result;
use chrono::{DateTime, Local};

#[derive(Debug)]
pub struct Author {
    name: String,
    email: String,
    time: DateTime<Local>,
}

impl Author {
    pub fn new(name: &str, email: &str, time: DateTime<Local>) -> Self {
        Author {
            name: name.to_string(),
            email: email.to_string(),
            time,
        }
    }

    pub fn parse(line: &str) -> Result<Self> {
        let mut iter = line.split_whitespace();
        let name = iter.next().unwrap();
        let email = iter.next().unwrap();
        let email = email.strip_prefix('<').unwrap().strip_suffix('>').unwrap();
        let time = iter.next().unwrap();
        let time_zone = iter.next().unwrap();
        let time = DateTime::parse_from_str(&format!("{} {}", time, time_zone), "%s %z")?;
        Ok(Author::new(name, email, time.into()))
    }

    pub fn to_s(&self) -> String {
        let timestampt = &self.time.format("%s %z").to_string();
        format!("{} <{}> {}", &self.name, &self.email, timestampt)
    }
}
