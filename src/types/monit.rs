use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct MonitInfo {
    pub id: Option<String>,
    pub name: Option<String>,
}

impl MonitInfo {
    pub fn new(id: Option<String>, name: Option<String>) -> Self {
        Self { id, name }
    }
}
