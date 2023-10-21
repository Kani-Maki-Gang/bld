use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LibvirtAuth {
    pub user: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibvirtConfig {
    pub uri: String,
    pub auth: Option<LibvirtAuth>,
}
