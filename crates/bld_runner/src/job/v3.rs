#[cfg(feature = "all")]
use crate::expr::v3::traits::ExprText;
use crate::{runs_on::v3::RunsOn, step::v3::Step, strategy::v3::Strategy};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[cfg(feature = "all")]
use {
    crate::{
        deps::v3::{Dependencies, Dependency},
        expr::v3::{
            parser::Rule,
            traits::{
                EvalObject, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
            },
        },
        strategy::v3::validate_matrix_refs,
        validator::v3::{Validate, ValidatorContext},
    },
    anyhow::{Result, bail},
    bld_core::fs::FileSystem,
    bld_pkg::PackageManager,
    pest::iterators::Pairs,
    std::iter::Peekable,
    tracing::debug,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Needs {
    Single(String),
    Multiple(HashSet<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(default = "Job::default_id")]
    pub id: String,
    pub runs_on: RunsOn,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub needs: Option<Needs>,
    #[serde(default = "Job::default_dispose")]
    pub dispose: bool,
    pub strategy: Option<Strategy>,
    pub steps: Vec<Step>,
}

impl Job {
    pub fn default_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn default_dispose() -> bool {
        true
    }
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            runs_on: RunsOn::default(),
            condition: None,
            needs: None,
            dispose: Self::default_dispose(),
            strategy: None,
            steps: vec![],
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Dependencies<'a> for Job {
    async fn local_deps(&'a self, fs: &FileSystem) -> Vec<Dependency<'a>> {
        let mut deps = vec![];
        for step in &self.steps {
            deps.append(&mut step.local_deps(fs).await);
        }
        deps
    }
    async fn remote_deps(&'a self, manager: &PackageManager) -> Vec<Dependency<'a>> {
        let mut deps = vec![];
        for step in &self.steps {
            deps.append(&mut step.remote_deps(manager).await);
        }
        deps
    }

    async fn jobs(&'a self) -> Vec<Dependency<'a>> {
        let Some(needs) = self.needs.as_ref() else {
            return vec![];
        };
        match needs {
            Needs::Single(need) => vec![Dependency::Job(need)],
            Needs::Multiple(need) => need.iter().map(|x| Dependency::Job(x.as_str())).collect(),
        }
    }

    async fn all(&'a self, manager: &PackageManager, fs: &FileSystem) -> Vec<Dependency<'a>> {
        let mut deps = self.local_deps(fs).await;
        deps.append(&mut self.remote_deps(manager).await);
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
        let Some(part) = path.next() else {
            bail!("no object path present");
        };
        let value = match part.as_span().as_str() {
            "runs_on" => self.runs_on.eval_object(path, rctx, wctx)?,

            "dispose" => ExprValue::Boolean(self.dispose),

            "matrix" => {
                let Some(part) = path.next() else {
                    bail!("expected name of matrix variable in object path");
                };
                let name = part.as_span().as_str();
                wctx.get_matrix_value(name)
                    .map(|x| ExprValue::Text(ExprText::Ref(x)))?
            }

            "steps" => {
                let Some(step_id) = path.next() else {
                    bail!("expected id for step in expression");
                };

                let step_id = step_id.as_span().as_str();

                let Some(step) = self.steps.iter().find(|x| x.is(step_id)) else {
                    bail!("step with id {step_id} not defined");
                };

                step.eval_object(path, rctx, wctx)?
            }

            value => bail!("invalid jobs field: {value}"),
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

        let job_matrix_keys: HashSet<&str> = self
            .strategy
            .as_ref()
            .map(|s| s.matrix_keys())
            .unwrap_or_default();

        if let Some(strategy) = self.strategy.as_ref() {
            debug!("Validating job's {} strategy section", self.id);
            ctx.push_section("strategy");
            strategy.validate(ctx).await;
            ctx.pop_section();
        }

        if let Some(condition) = self.condition.as_ref() {
            debug!("Validating job's {} if condition", self.id);
            ctx.push_section("if");
            if ctx.expression_count(condition) > 1 {
                ctx.append_error("Condition must contain at most one expression");
            } else {
                ctx.validate_expressions(condition);
            }
            validate_matrix_refs(ctx, condition, &HashSet::new());
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
            step.validate_matrix(ctx, Some(&job_matrix_keys)).await;
        }
        ctx.pop_section();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bld_config::BldConfig;
    use bld_core::fs::FileSystem;
    use bld_pkg::PackageManager;
    use bld_utils::sync::IntoArc;

    use crate::{
        expr::v3::context::CommonReadonlyRuntimeExprContext,
        pipeline::v3::Pipeline,
        step::v3::{ShellCommand, Step},
        strategy::v3::{MatrixValue, Strategy},
        validator::v3::{CommonValidator, ConsumeValidator, ValidatorWritableRuntimeExprContext},
    };

    use super::Job;

    async fn validate_job(job: Job) -> anyhow::Result<()> {
        let job_name = "main";
        let config = BldConfig::default().into_arc();
        let file_system = FileSystem::local(config.clone()).into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default();
        let expr_wctx = vec![ValidatorWritableRuntimeExprContext::new(job_name)];

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(job_name.to_string(), job);

        CommonValidator::new(
            &pipeline,
            config,
            file_system,
            package_manager,
            &expr_rctx,
            &expr_wctx,
        )
        .unwrap()
        .validate()
        .await
    }

    fn matrix_of(values: Vec<(&str, Vec<&str>)>) -> HashMap<String, MatrixValue> {
        values
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    MatrixValue::Array(v.into_iter().map(|x| x.to_string()).collect()),
                )
            })
            .collect()
    }

    #[tokio::test]
    pub async fn matrix_ref_with_job_and_step_keys_success() {
        let job = Job {
            strategy: Some(Strategy {
                matrix: matrix_of(vec![("os", vec!["linux", "windows"])]),
                fail_fast: None,
            }),
            steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                id: "build".to_string(),
                run: "echo ${{ matrix.os }} ${{ matrix.version }}".to_string(),
                strategy: Some(Strategy {
                    matrix: matrix_of(vec![("version", vec!["v2", "v3"])]),
                    fail_fast: None,
                }),
                ..Default::default()
            }))],
            ..Default::default()
        };

        let result = validate_job(job).await;
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[tokio::test]
    pub async fn matrix_ref_undefined_key_failure() {
        let job = Job {
            steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                id: "build".to_string(),
                run: "echo ${{ matrix.missing }}".to_string(),
                ..Default::default()
            }))],
            ..Default::default()
        };

        let result = validate_job(job).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    pub async fn matrix_duplicate_key_between_job_and_step_failure() {
        let job = Job {
            strategy: Some(Strategy {
                matrix: matrix_of(vec![("os", vec!["linux", "windows"])]),
                fail_fast: None,
            }),
            steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                id: "build".to_string(),
                run: "echo ${{ matrix.os }}".to_string(),
                strategy: Some(Strategy {
                    matrix: matrix_of(vec![("os", vec!["mac"])]),
                    fail_fast: None,
                }),
                ..Default::default()
            }))],
            ..Default::default()
        };

        let result = validate_job(job).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    pub async fn matrix_non_array_value_failure() {
        let mut matrix = HashMap::new();
        matrix.insert("os".to_string(), MatrixValue::Expr("${{ 5 }}".to_string()));

        let job = Job {
            strategy: Some(Strategy {
                matrix,
                fail_fast: None,
            }),
            steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                id: "build".to_string(),
                run: "echo ${{ matrix.os }}".to_string(),
                ..Default::default()
            }))],
            ..Default::default()
        };

        let result = validate_job(job).await;
        assert!(result.is_err());
    }
}
