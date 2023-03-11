use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Platform {
    #[default]
    #[serde(rename(serialize = "machine", deserialize = "machine"))]
    Machine,
    Image(String),
    Dockerfile {
        image: String,
        dockerfile: String,
        tag: Option<String>,
        #[serde(default)]
        rebuild: bool,
    },
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Machine => write!(f, "machine"),
            Self::Image(image) => write!(f, "{image}"),
            Self::Dockerfile { image, .. } => write!(f, "{image}"),
        }
    }
}
