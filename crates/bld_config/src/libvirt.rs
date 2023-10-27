use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LibvirtConfig {
    pub uri: String,
    pub domain: String,
    pub start_before_run: Option<String>,
    pub shutdown_after_run: Option<String>,
}
