use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Platform {
    #[default]
    #[serde(rename(serialize = "machine", deserialize = "machine"))]
    Machine,
    Container(String),
    ContainerByPull {
        image: String,
        pull: bool,
    },
    ContainerByBuild {
        tag: String,
        dockerfile: String,
    },
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Machine => write!(f, "machine"),
            Self::Container(image) => write!(f, "{image}"),
            Self::ContainerByPull { image, .. } => write!(f, "{image}"),
            Self::ContainerByBuild { tag, .. } => write!(f, "{tag}"),
        }
    }
}
