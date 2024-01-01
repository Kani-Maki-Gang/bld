use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::definitions;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DockerUrl {
    SingleUrl(String),
    MultipleUrls(HashMap<String, String>),
}

impl Default for DockerUrl {
    fn default() -> Self {
        Self::SingleUrl(definitions::LOCAL_DOCKER_URL.to_owned())
    }
}
