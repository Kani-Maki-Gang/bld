use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct External {
    pub name: String,
    pub server: Option<String>,
    pub pipeline: String,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub environment: HashMap<String, String>,
}
