use std::{
    collections::{HashMap, HashSet},
    iter::Peekable,
};

use anyhow::Result;
use pest::iterators::Pairs;
use serde::{Deserialize, Serialize};

use crate::{
    expr::v3::{
        parser::Rule,
        traits::{EvalObject, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext},
    },
    inputs::v3::Input,
    step::v3::Step,
    traits::{IntoVariables, Variables},
};

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use crate::{
    traits::Dependencies,
    validator::v3::{Validate, ValidatorContext},
};

#[cfg(feature = "all")]
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,

    #[serde(default)]
    pub inputs: HashMap<String, Input>,

    #[serde(default)]
    pub steps: Vec<Step>,
}

impl Action {
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

    pub fn required_inputs(&self) -> Option<HashSet<&str>> {
        if !self.inputs.is_empty() {
            let map = self
                .inputs
                .iter()
                .filter(|(_, v)| v.is_required())
                .map(|(k, _)| k.as_str())
                .collect();
            Some(map)
        } else {
            None
        }
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
        (Some(inputs), None)
    }
}

#[cfg(feature = "all")]
impl Dependencies for Action {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        self.steps
            .iter()
            .flat_map(|s| s.local_deps(config))
            .collect()
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Action {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        _path: &mut Peekable<Pairs<'_, Rule>>,
        _rctx: &RCtx,
        _wctx: &WCtx,
    ) -> Result<ExprValue<'a>> {
        unimplemented!()
    }
}

#[cfg(feature = "all")]
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
