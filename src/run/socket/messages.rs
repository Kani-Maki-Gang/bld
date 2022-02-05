use actix::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub struct ExecInfo {
    pub name: String,
    pub variables: Option<HashMap<String, String>>,
}

impl ExecInfo {
    pub fn new(name: &str, variables: Option<HashMap<String, String>>) -> Self {
        Self {
            name: name.to_string(),
            variables,
        }
    }
}
