use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PushInfo {
    pub name: String,
    pub content: String,
}

impl PushInfo {
    pub fn new(name: &str, content: &str) -> Self {
        PushInfo {
            name: name.to_string(),
            content: content.to_string(),
        }
    }
}
