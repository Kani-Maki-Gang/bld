use serde::{Deserialize, Serialize};
use actix::Message;

#[derive(Default, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct MonitInfo {
    pub id: Option<String>,
    pub name: Option<String>,
}

impl MonitInfo {
    pub fn new(id: Option<String>, name: Option<String>) -> Self {
        Self { id, name }
    }
}
