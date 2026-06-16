use crate::{runs_on::v3::RunsOn, step::v3::Step};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "all")]
use {
    crate::{
        deps::v3::{Dependencies, Dependency},
        expr::v3::{
            parser::Rule,
            traits::{
                EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext,
                WritableRuntimeExprContext,
            },
        },
        validator::v3::{Validate, ValidatorContext},
    },
    anyhow::{Result, bail},
    bld_core::fs::FileSystem,
    bld_pkg::PackageManager,
    pest::iterators::Pairs,
    std::{collections::HashSet, iter::Peekable},
    tracing::debug,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Needs {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(default = "Job::default_id")]
    pub id: String,
    pub runs_on: RunsOn,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub needs: Option<Needs>,
    pub steps: Vec<Step>,
}

impl Job {
    pub fn default_id() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            runs_on: RunsOn::default(),
            condition: None,
            needs: None,
            steps: vec![],
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Dependencies<'a> for Job {
    async fn local_deps(
        &'a self,
        manager: &PackageManager,
        _fs: &FileSystem,
    ) -> Vec<Dependency<'a>> {
        let mut deps = vec![];
        for step in &self.steps {
            deps.append(&mut step.local_deps(manager).await);
        }
        deps
    }
    async fn remote_deps(&'a self) -> Vec<Dependency<'a>> {
        let mut deps = vec![];
        for step in &self.steps {
            deps.append(&mut step.remote_deps().await);
        }
        deps
    }

    async fn jobs(&'a self) -> Vec<Dependency<'a>> {
        unimplemented!();
    }

    async fn all(&'a self, manager: &PackageManager) -> Vec<Dependency<'a>> {
        let mut deps = self.local_deps(manager).await;
        deps.append(&mut self.remote_deps().await);
        deps.append(&mut self.jobs().await);
        deps
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Job {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };

        let mut object_parts = object.into_inner();
        let Some(part) = object_parts.next() else {
            bail!("expected at least one part in the object path");
        };

        let value = match part.as_span().as_str() {
            "runs_on" => self
                .runs_on
                .eval_object(&mut object_parts.peekable(), rctx, wctx)?,

            "steps" => {
                let Some(step_id) = object_parts.next() else {
                    bail!("expected id for step in expression");
                };

                let step_id = step_id.as_span().as_str();

                let Some(step) = self.steps.iter().find(|x| x.is(step_id)) else {
                    bail!("step with id {step_id} not defined");
                };

                step.eval_object(&mut object_parts.peekable(), rctx, wctx)?
            }

            "outputs" => {
                let Some(object) = path.next() else {
                    bail!("no output variable name provided");
                };
                let name = object.as_span().as_str();
                ExprValue::Text(ExprText::Ref(wctx.get_output(&self.id, name)?))
            }

            value => bail!("invalid steps field: {value}"),
        };

        Ok(value)
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Job {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating job {}", self.id);

        debug!("Validating job's {} runs_on section", self.id);
        ctx.push_section("runs_on");
        self.runs_on.validate(ctx).await;
        ctx.pop_section();

        if let Some(condition) = self.condition.as_ref() {
            debug!("Validating job's {} if condition", self.id);
            ctx.push_section("if");
            if ctx.expression_count(condition) > 1 {
                ctx.append_error("Condition must contain at most one expression");
            } else {
                ctx.validate_expressions(condition);
            }
            ctx.pop_section();
        }

        debug!("Validating job's {} steps", self.id);
        ctx.push_section("steps");
        if self.steps.is_empty() {
            ctx.append_error("Pipeline must have at least one job defined");
        }

        let mut step_ids = HashSet::new();
        for step in &self.steps {
            let step_id = step.id();
            if !step_ids.insert(step_id) {
                ctx.push_section(step_id);
                ctx.append_error(&format!("Duplicate step id '{step_id}' found in job"));
                ctx.pop_section();
            }
            step.validate(ctx).await;
        }
        ctx.pop_section();
    }
}
