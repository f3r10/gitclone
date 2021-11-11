use chrono::{DateTime, Local};

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
            time: time,
        }
    }

    pub fn to_s(&self) -> String {
        let timestampt = &self.time.format("%s %z").to_string();
        format!("{} <{}> {}", &self.name, &self.email, timestampt)
    }
}
