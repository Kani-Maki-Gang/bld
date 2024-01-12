use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::definitions;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DockerUrlEntry {
    Url(String),
    UrlWithDefault { url: String, default: bool },
}

impl DockerUrlEntry {
    pub fn is_default(&self) -> bool {
        matches!(self, Self::UrlWithDefault { default: true, .. })
    }

    pub fn get_url_with_default(&self) -> Result<&str> {
        match self {
            Self::UrlWithDefault { url, .. } => Ok(url),
            _ => bail!("unable to retrieve docker url from config"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DockerUrl {
    Single(String),
    Multiple(HashMap<String, DockerUrlEntry>),
}

impl Default for DockerUrl {
    fn default() -> Self {
        Self::Single(definitions::LOCAL_DOCKER_URL.to_owned())
    }
}
