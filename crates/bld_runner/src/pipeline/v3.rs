use crate::external::v3::External;
use crate::runs_on::v3::RunsOn;
use crate::step::v3::BuildStep;
use crate::traits::Variables;
use crate::validator::v3::{Validatable, ValidatorContext};
use crate::{artifacts::v3::Artifacts, traits::IntoVariables};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

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
        (self.inputs, self.env)
    }
}

impl<'a> Validatable<'a> for Pipeline {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        ctx.push_section("runs_on");
        self.runs_on.validate(ctx).await;
        ctx.pop_section();

        self.validate_cron(ctx);

        ctx.push_section("inputs");
        ctx.validate_inputs(&self.inputs);
        ctx.pop_section();

        ctx.push_section("env");
        ctx.validate_env(&self.env);
        ctx.pop_section();

        ctx.push_section("external");
        for external in &self.external {
            external.validate(ctx).await;
        }
        ctx.pop_section();

        ctx.push_section("external");
        for artifact in &self.artifacts {
            artifact.validate(ctx).await;
        }
        ctx.pop_section();

        ctx.push_section("jobs");
        for (job, steps) in &self.jobs {
            ctx.push_section(job);
            for step in steps {
                step.validate(ctx).await;
            }
            ctx.pop_section();
        }
        ctx.pop_section();
    }
}
