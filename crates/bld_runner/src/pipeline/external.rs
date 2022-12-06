use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExternalV1 {
    pub name: Option<String>,
    pub server: Option<String>,
    pub pipeline: String,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub environment: HashMap<String, String>,
}

impl ExternalV1 {
    pub fn is(&self, value: &str) -> bool {
        self.name.as_ref().map(|n| n == value).unwrap_or_default() || self.pipeline == value
    }

    pub fn local(pipeline: &str) -> Self {
        ExternalV1 {
            pipeline: pipeline.to_owned(),
            ..Default::default()
        }
    }
}
