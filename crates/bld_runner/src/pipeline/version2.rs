use crate::artifacts::version2::Artifacts;
use crate::external::version2::External;
use crate::platform::version2::Platform;
use crate::step::version2::BuildStep;
use crate::token_context::version2::PipelineContext;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::traits::{ApplyTokens, CompleteTokenTransformer};

#[derive(Debug, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: Platform,

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

#[async_trait]
impl<'a> ApplyTokens<'a, PipelineContext<'a>> for Pipeline {
    async fn apply_tokens(&mut self, context: &'a PipelineContext<'a>) -> Result<()> {
        self.runs_on.apply_tokens(context).await?;

        self.dispose = context
            .transform(self.dispose.to_string())
            .await?
            .parse::<bool>()?;

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

        for entry in self.steps.iter_mut() {
            entry.apply_tokens(context).await?;
        }

        Ok(())
    }
}
