use crate::artifacts::v1::Artifacts;
use crate::external::v1::External;
use crate::step::v1::BuildStep;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: String,

    #[serde(default = "Pipeline::default_dispose")]
    pub dispose: bool,

    #[serde(default)]
    pub environment: HashMap<String, String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub artifacts: Vec<Artifacts>,

    #[serde(default)]
    pub external: Vec<External>,

    #[serde(default)]
    pub steps: Vec<BuildStep>,
}

impl Pipeline {
    fn default_dispose() -> bool {
        true
    }

    #[cfg(feature = "all")]
    pub fn local_dependencies(&self, config: &BldConfig) -> Vec<String> {
        let from_steps = self.steps.iter().flat_map(|s| s.local_dependencies(config));

        let from_external = self
            .external
            .iter()
            .filter(|e| e.server.is_none())
            .map(|e| e.pipeline.to_owned());

        from_steps.chain(from_external).collect()
    }
}
