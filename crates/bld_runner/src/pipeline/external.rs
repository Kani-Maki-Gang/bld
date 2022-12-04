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
    pub fn local(pipeline: &str) -> Self {
        let mut ext = ExternalV1::default();
        ext.pipeline = pipeline.to_owned();
        ext
    }
}
