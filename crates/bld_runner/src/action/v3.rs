use std::collections::HashMap;

use bld_config::BldConfig;
use serde::{Deserialize, Serialize};

use crate::{
    step::v3::BuildStep,
    traits::{Dependencies, IntoVariables, Variables},
    validator::v3::{Validatable, ValidatorContext},
};

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;

#[cfg(feature = "all")]
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,

    #[serde(default)]
    pub inputs: HashMap<String, String>,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub steps: Vec<BuildStep>,
}

impl Action {
    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &'a ExecutionContext<'a>) -> Result<()> {
        for (_, v) in self.env.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        for (_, v) in self.inputs.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        for step in self.steps.iter_mut() {
            step.apply_tokens(context).await?;
        }

        Ok(())
    }
}

impl Dependencies for Action {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        self.steps
            .iter()
            .flat_map(|s| s.local_deps(config))
            .collect()
    }
}

impl IntoVariables for Action {
    fn into_variables(self) -> Variables {
        (self.inputs, self.env)
    }
}

impl<'a> Validatable<'a> for Action {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        ctx.push_section("inputs");
        ctx.validate_inputs(&self.inputs);
        ctx.pop_section();

        ctx.push_section("env");
        ctx.validate_env(&self.env);
        ctx.pop_section();

        ctx.push_section("steps");
        for step in &self.steps {
            step.validate(ctx).await;
        }
        ctx.pop_section();
    }
}
