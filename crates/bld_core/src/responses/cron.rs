use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CronJobResponse {
    pub id: String,
    pub schedule: String,
    pub pipeline: String,
    pub variables: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
    pub is_default: bool,
    pub date_created: String,
    pub date_updated: Option<String>,
}
