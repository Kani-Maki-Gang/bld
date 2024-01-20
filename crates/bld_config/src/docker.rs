use anyhow::{anyhow, bail, Result};
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

impl DockerUrl {
    pub fn get_url_or_default<'a>(&'a self, name: Option<&'a str>) -> Result<&'a str> {
        match (&self, name) {
            (DockerUrl::Single(url), _) => Ok(url),

            (DockerUrl::Multiple(urls), Some(name)) => {
                let (DockerUrlEntry::Url(url) | DockerUrlEntry::UrlWithDefault { url, .. }) = urls
                    .get(name)
                    .ok_or_else(|| anyhow!("unable to find docker url entry in config"))?;
                Ok(&url)
            }

            (DockerUrl::Multiple(urls), None) => {
                let instances: Vec<&str> = urls
                    .iter()
                    .filter(|(_, x)| x.is_default())
                    .flat_map(|(_, x)| x.get_url_with_default())
                    .collect();

                if instances.len() > 1 {
                    bail!("multiple default docker urls defined in config");
                }

                instances
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("no default docker url defined in config"))
            }
        }
    }
}

impl Default for DockerUrl {
    fn default() -> Self {
        Self::Single(definitions::LOCAL_DOCKER_URL.to_owned())
    }
}
