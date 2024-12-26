use std::collections::{HashMap, HashSet};

use bld_config::BldConfig;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    inputs::v3::Input,
    step::v3::Step,
    traits::{Dependencies, IntoVariables, Variables},
    validator::v3::{Validate, ValidatorContext},
};

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;

#[cfg(feature = "all")]
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,

    #[serde(default)]
    pub inputs: HashMap<String, Input>,

    #[serde(default)]
    pub steps: Vec<Step>,
}

impl Action {
    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &'a ExecutionContext<'a>) -> Result<()> {
        for (_name, input) in self.inputs.iter_mut() {
            input.apply_tokens(context).await?;
        }

        for step in self.steps.iter_mut() {
            step.apply_tokens(context).await?;
        }

        Ok(())
    }

    pub fn inputs_map(&self) -> HashMap<String, String> {
        let mut inputs = HashMap::new();
        for (name, input) in &self.inputs {
            match input {
                Input::Simple(v) => {
                    inputs.insert(name.to_owned(), v.to_owned());
                }
                Input::Complex { default, .. } => {
                    inputs.insert(name.to_owned(), default.to_owned().unwrap_or_default());
                }
            }
        }
        inputs
    }

    pub fn required_inputs(&self) -> HashSet<&str> {
        self.inputs
            .iter()
            .filter(|(_, v)| v.is_required())
            .map(|(k, _)| k.as_str())
            .collect()
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
        let mut inputs = HashMap::new();
        for (name, input) in self.inputs {
            match input {
                Input::Simple(v) => {
                    inputs.insert(name, v);
                }
                Input::Complex { default, .. } => {
                    inputs.insert(name, default.unwrap_or_default());
                }
            }
        }
        (inputs, HashMap::new())
    }
}

impl<'a> Validate<'a> for Action {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating action: {}", self.name);

        debug!("Validating action's inputs section");
        ctx.push_section("inputs");
        for (name, input) in self.inputs.iter() {
            debug!("Validating input: {}", name);
            ctx.push_section(name);
            ctx.validate_keywords(name);
            input.validate(ctx).await;
            ctx.pop_section();
        }
        ctx.pop_section();

        debug!("Validating action's steps");
        ctx.push_section("steps");
        for (i, step) in self.steps.iter().enumerate() {
            debug!("Validating step at index {i}");
            step.validate(ctx).await;
        }
        ctx.pop_section();
    }
}
