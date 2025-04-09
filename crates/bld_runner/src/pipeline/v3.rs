use crate::{
    artifacts::v3::Artifacts,
    expr::v3::traits::{ExprText, RuntimeExecutionContext},
    external::v3::External,
    inputs::v3::Input,
    runs_on::v3::RunsOn,
    step::v3::Step,
    traits::{IntoVariables, Variables},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    iter::Peekable,
};

#[cfg(feature = "all")]
use {
    crate::{
        expr::v3::{
            parser::Rule,
            traits::{EvalObject, ExprValue},
        },
        token_context::v3::{ApplyContext, ExecutionContext},
        validator::v3::{Validate, ValidatorContext},
    },
    anyhow::{Result, anyhow, bail},
    cron::Schedule,
    pest::iterators::Pairs,
    std::str::FromStr,
    tracing::debug,
};

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

    pub fn required_inputs(&self) -> Option<HashSet<&str>> {
        if !self.inputs.is_empty() {
            let inputs = self
                .inputs
                .iter()
                .filter(|(_, v)| v.is_required())
                .map(|(k, _)| k.as_str())
                .collect();
            Some(inputs)
        } else {
            None
        }
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
        let mut inputs: Option<HashMap<String, String>> = None;

        if !self.inputs.is_empty() {
            let map = self
                .inputs
                .into_iter()
                .map(|(name, input)| match input {
                    Input::Simple(v) => (name, v),
                    Input::Complex { default, .. } => (name, default.unwrap_or_default()),
                })
                .collect();
            inputs = Some(map);
        }

        (inputs, Some(self.env))
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
impl<'a> EvalObject<'a> for Pipeline {
    fn eval_object<Ctx: RuntimeExecutionContext<'a>>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        ctx: &Ctx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };

        let Rule::Object = object.as_rule() else {
            bail!("expected object path");
        };

        let mut object_parts = object.into_inner();
        let Some(part) = object_parts.next() else {
            bail!("expected at least one part in the object path");
        };

        match part.as_span().as_str() {
            "name" => {
                let name = self.name.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(name)))
            }

            "runs_on" => self.runs_on.eval_object(&mut object_parts.peekable(), ctx),

            "dispose" => Ok(ExprValue::Boolean(self.dispose)),

            "cron" => {
                let cron = self.cron.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(cron)))
            }

            "inputs" => {
                let Some(part) = object_parts.next() else {
                    bail!("expected name of input in object path");
                };
                let name = part.as_span().as_str();
                let input = self
                    .inputs
                    .get(name)
                    .ok_or_else(|| anyhow!("input '{name}' not found"))?;
                input.try_into().map(|x| ExprValue::Text(ExprText::Ref(x)))
            }

            "env" => {
                let Some(part) = object_parts.next() else {
                    bail!("expected name of env variable in object path");
                };
                let name = part.as_span().as_str();
                self.env
                    .get(name)
                    .map(|x| ExprValue::Text(ExprText::Ref(x)))
                    .ok_or_else(|| anyhow!("env variable '{name}' not found"))
            }

            "jobs" => unimplemented!(),

            _ => unimplemented!(),
        }
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
