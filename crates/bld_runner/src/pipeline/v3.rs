use crate::{
    artifacts::v3::Artifacts,
    external::v3::External,
    inputs::v3::Input,
    runs_on::v3::RunsOn,
    step::v3::Step,
    traits::{IntoVariables, Variables},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(feature = "all")]
use crate::{
    token_context::v3::{ApplyContext, ExecutionContext},
    validator::v3::{Validate, ValidatorContext},
};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use std::str::FromStr;

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
use cron::Schedule;

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
    pub inputs: HashMap<String, Input>,

    #[serde(default)]
    pub artifacts: Vec<Artifacts>,

    #[serde(default)]
    pub external: Vec<External>,

    #[serde(default)]
    pub jobs: HashMap<String, Vec<Step>>,
}

impl Pipeline {
    fn default_dispose() -> bool {
        true
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

    #[cfg(feature = "all")]
    fn validate_cron<'a, C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        let Some(cron) = self.cron.as_ref() else {
            return;
        };
        if let Err(e) = Schedule::from_str(cron) {
            let error = format!("{cron} {e}");
            ctx.push_section("cron");
            ctx.append_error(&error);
            ctx.pop_section();
        }
    }
}

impl IntoVariables for Pipeline {
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
        (inputs, self.env)
    }
}

#[cfg(feature = "all")]
impl ApplyContext for Pipeline {
    async fn apply_context<C: ExecutionContext>(&mut self, ctx: &C) -> Result<()> {
        self.runs_on.apply_context(ctx).await?;

        for (_, v) in self.env.iter_mut() {
            *v = ctx.transform(v.to_owned()).await?;
        }

        for (_name, input) in self.inputs.iter_mut() {
            input.apply_context(ctx).await?;
        }

        for entry in self.external.iter_mut() {
            entry.apply_context(ctx).await?;
        }

        for entry in self.artifacts.iter_mut() {
            entry.apply_context(ctx).await?;
        }

        for step in self.jobs.iter_mut().flat_map(|(_, steps)| steps) {
            step.apply_context(ctx).await?;
        }

        Ok(())
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Pipeline {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating pipeline");

        debug!("Validating pipeline's runs_on section");
        ctx.push_section("runs_on");
        self.runs_on.validate(ctx).await;
        ctx.pop_section();

        debug!("Validating pipeline's cron value");
        self.validate_cron(ctx);

        debug!("Validating pipeline's inputs section");
        ctx.push_section("inputs");
        for (name, input) in self.inputs.iter() {
            debug!("Validating input: {}", name);
            ctx.push_section(name);
            ctx.validate_keywords(name);
            input.validate(ctx).await;
            ctx.pop_section();
        }
        ctx.pop_section();

        debug!("Validating pipeline's env section");
        ctx.push_section("env");
        ctx.validate_env(&self.env);
        ctx.pop_section();

        debug!("Validating pipeline external section");
        ctx.push_section("external");
        for external in &self.external {
            external.validate(ctx).await;
        }
        ctx.pop_section();

        debug!("Validating pipeline's artifacts section");
        ctx.push_section("artifacts");
        for (i, artifact) in self.artifacts.iter().enumerate() {
            debug!("Validating artifact at index {i}");
            artifact.validate(ctx).await;
        }
        ctx.pop_section();

        debug!("Validating pipeline's jobs section");
        ctx.push_section("jobs");
        for (job, steps) in &self.jobs {
            ctx.push_section(job);
            debug!("Validating {job} job's steps");
            for step in steps {
                step.validate(ctx).await;
            }
            ctx.pop_section();
        }
        ctx.pop_section();
    }
}
