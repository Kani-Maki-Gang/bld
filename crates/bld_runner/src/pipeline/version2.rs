use crate::artifacts::version2::Artifacts;
use crate::external::version2::External;
use crate::platform::version2::Platform;
use crate::step::version2::BuildStep;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: Platform,

    #[serde(default)]
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
    pub fn local_dependencies(&self) -> Vec<String> {
        let from_steps = self.steps.iter().flat_map(|s| s.local_dependencies());

        let from_external = self
            .external
            .iter()
            .filter(|e| e.server.is_none())
            .map(|e| e.pipeline.to_owned());

        from_steps.chain(from_external).collect()
    }
}
