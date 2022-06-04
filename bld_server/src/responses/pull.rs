use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PullResponse {
    pub name: String,
    pub content: String,
}

impl PullResponse {
    pub fn new(name: &str, content: &str) -> Self {
        Self {
            name: name.to_string(),
            content: content.to_string(),
        }
    }
}
