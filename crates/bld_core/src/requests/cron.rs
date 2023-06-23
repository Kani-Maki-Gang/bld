use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CronRequest {
    pub schedule: String,
    pub pipeline: String,
    pub variables: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
}
