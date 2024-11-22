use crate::traits::Variables;
use crate::validator::v3::{ErrorBuilder, Validatable};
use crate::{artifacts::v3::Artifacts, traits::IntoVariables};
use crate::external::v3::External;
use crate::runs_on::v3::RunsOn;
use crate::step::v3::BuildStep;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;

#[cfg(feature = "all")]
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunsOn,
    pub cron: Option<String>,

    #[serde(default = "Pipeline::default_dispose")]
    pub dispose: bool,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub inputs: HashMap<String, String>,

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
    pub async fn apply_tokens<'a>(&mut self, context: &'a ExecutionContext<'a>) -> Result<()> {
        self.runs_on.apply_tokens(context).await?;

        for (_, v) in self.env.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        for (_, v) in self.inputs.iter_mut() {
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
        (self.inputs, self.env)
    }
}

impl Validatable for Pipeline {
    async fn validate(&self, error_builder: &mut impl ErrorBuilder) {
        unimplemented!()
    }
}
