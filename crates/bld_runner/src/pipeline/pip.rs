use super::artifacts::ArtifactsV1;
use super::external::ExternalV1;
use super::step::BuildStepV1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PipelineV1 {
    pub name: Option<String>,
    pub runs_on: String,

    #[serde(default = "PipelineV1::default_dispose")]
    pub dispose: bool,

    #[serde(default)]
    pub environment: HashMap<String, String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub artifacts: Vec<ArtifactsV1>,

    #[serde(default)]
    pub external: Vec<ExternalV1>,

    #[serde(default)]
    pub steps: Vec<BuildStepV1>,
}

impl PipelineV1 {
    fn default_dispose() -> bool {
        true
    }

    pub fn local_dependencies(&self) -> Vec<String> {
        self.external
            .iter()
            .filter(|e| e.server.is_none())
            .map(|e| e.pipeline.to_owned())
            .collect()
    }
}
