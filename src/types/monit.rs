use serde::{Deserialize, Serialize};
use actix::Message;

#[derive(Default, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct MonitInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub last: bool,
}

impl MonitInfo {
    pub fn new(id: Option<String>, name: Option<String>, last: bool) -> Self {
        Self { id, name, last }
    }
}
