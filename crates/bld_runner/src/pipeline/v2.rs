use crate::external::v2::External;
use crate::runs_on::v2::RunsOn;
use crate::step::v2::BuildStep;
use crate::traits::Variables;
use crate::{artifacts::v2::Artifacts, traits::IntoVariables};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "all")]
use crate::token_context::v2::PipelineContext;

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use crate::traits::Dependencies;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunsOn,

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

    #[cfg(feature = "all")]
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

impl IntoVariables for Pipeline {
    fn into_variables(self) -> Variables {
        (Some(self.variables), Some(self.environment))
    }
}

#[cfg(feature = "all")]
impl Dependencies for Pipeline {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        let from_steps = self
            .jobs
            .iter()
            .flat_map(|(_, steps)| steps)
            .flat_map(|s| s.local_dependencies(config));

        let from_external = self
            .external
            .iter()
            .filter(|e| e.server.is_none())
            .map(|e| e.pipeline.to_owned());

        from_steps.chain(from_external).collect()
    }
}
