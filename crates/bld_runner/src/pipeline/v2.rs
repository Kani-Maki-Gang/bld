use crate::artifacts::v2::Artifacts;
use crate::external::v2::External;
use crate::platform::v2::Platform;
use crate::step::v2::BuildStep;
use crate::token_context::v2::PipelineContext;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: Platform,

    pub cron: Option<String>,

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
    pub jobs: HashMap<String, Vec<BuildStep>>,
}

impl Pipeline {
    fn default_dispose() -> bool {
        true
    }

    pub fn local_dependencies(&self) -> Vec<String> {
        let from_steps = self
            .jobs
            .iter()
            .flat_map(|(_, steps)| steps)
            .flat_map(|s| s.local_dependencies());

        let from_external = self
            .external
            .iter()
            .filter(|e| e.server.is_none())
            .map(|e| e.pipeline.to_owned());

        from_steps.chain(from_external).collect()
    }

    pub async fn apply_tokens<'a>(&mut self, context: &'a PipelineContext<'a>) -> Result<()> {
        self.runs_on.apply_tokens(context).await?;

        for (_, v) in self.environment.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        for (_, v) in self.variables.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        for entry in self.external.iter_mut() {
            entry.apply_tokens(context).await?;
        }

        for entry in self.artifacts.iter_mut() {
            entry.apply_tokens(context).await?;
        }

        for step in self.jobs.iter_mut().flat_map(|(_, steps)| steps) {
            step.apply_tokens(context).await?;
        }

        Ok(())
    }
}
