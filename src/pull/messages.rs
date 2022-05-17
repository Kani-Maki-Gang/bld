use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub name: String,
    pub include_deps: bool,
}

impl PullRequestInfo {
    pub fn new(name: &str, include_deps: bool) -> Self {
        Self {
            name: name.to_string(),
            include_deps,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PullResponseInfo {
    pub name: String,
    pub content: String,
}

impl PullResponseInfo {
    pub fn new(name: &str, content: &str) -> Self {
        Self {
            name: name.to_string(),
            content: content.to_string(),
        }
    }
}
