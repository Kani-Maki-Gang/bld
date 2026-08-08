use crate::{
    artifacts::v3::{DownloadArtifact, UploadArtifact},
    external::v3::External,
    strategy::v3::Strategy,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "all")]
use {
    crate::{
        deps::v3::{Dependencies, Dependency, RemoteDependency},
        expr::v3::{
            parser::Rule,
            traits::{
                EvalObject, ExprText, ExprValue, OutputScope, ReadonlyRuntimeExprContext,
                WritableRuntimeExprContext,
            },
        },
        strategy::v3::validate_matrix_refs,
        validator::v3::{ExprScope, Validate, ValidatorContext},
    },
    anyhow::{Result, bail},
    bld_core::fs::FileSystem,
    bld_pkg::PackageManager,
    bld_utils::fs::IsYaml,
    pest::iterators::Pairs,
    std::collections::HashSet,
    std::iter::Peekable,
    tracing::debug,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    #[serde(default = "ShellCommand::default_id")]
    pub id: String,
    pub name: Option<String>,
    pub working_dir: Option<String>,
    pub run: String,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub strategy: Option<Strategy>,
}

impl ShellCommand {
    fn default_id() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for ShellCommand {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            name: None,
            working_dir: None,
            run: String::new(),
            condition: None,
            strategy: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
    ComplexSh(Box<ShellCommand>),
    ExternalFile(Box<External>),
    DownloadArtifact(Box<DownloadArtifact>),
    UploadArtifact(Box<UploadArtifact>),
}

impl Step {
    pub fn is(&self, id: &str) -> bool {
        self.id() == id
    }

    pub fn id(&self) -> &str {
        match self {
            Self::ComplexSh(cmd) => &cmd.id,
            Self::ExternalFile(ext) => &ext.id,
            Self::DownloadArtifact(value) => &value.id,
            Self::UploadArtifact(value) => &value.id,
        }
    }

    pub fn strategy(&self) -> Option<&Strategy> {
        match self {
            Self::ComplexSh(cmd) => cmd.strategy.as_ref(),
            Self::ExternalFile(ext) => ext.strategy.as_ref(),
            Self::DownloadArtifact(_) => None,
            Self::UploadArtifact(_) => None,
        }
    }

    #[cfg(feature = "all")]
    pub async fn validate_matrix<'a, C: ValidatorContext<'a>>(
        &'a self,
        ctx: &mut C,
        job_matrix_keys: Option<&HashSet<&'a str>>,
    ) {
        let step_id = self.id();
        ctx.push_section(step_id);

        let mut available: HashSet<&str> = job_matrix_keys.cloned().unwrap_or_default();

        if let Some(strategy) = self.strategy() {
            debug!("Validating step's {} strategy section", step_id);
            ctx.push_section("strategy");
            strategy.validate(ctx).await;
            ctx.pop_section();

            let self_matrix_keys = strategy.matrix_keys();

            if let Some(job_matrix_keys) = job_matrix_keys {
                for conflict_key in job_matrix_keys
                    .iter()
                    .filter(|x| self_matrix_keys.contains(*x))
                {
                    ctx.push_section("strategy");
                    ctx.push_section("matrix");
                    ctx.append_error(&format!(
                        "Matrix key '{conflict_key}' is already defined in the job's strategy"
                    ));
                    ctx.pop_section();
                    ctx.pop_section();
                }
            }

            available.extend(self_matrix_keys.iter());
        }

        for value in self.expr_field_values() {
            validate_matrix_refs(ctx, value, &available);
        }

        ctx.pop_section();
    }

    #[cfg(feature = "all")]
    fn expr_field_values(&self) -> Vec<&str> {
        match self {
            Step::ComplexSh(cmd) => {
                let mut values = vec![cmd.run.as_str()];
                if let Some(name) = cmd.name.as_deref() {
                    values.push(name);
                }
                if let Some(wd) = cmd.working_dir.as_deref() {
                    values.push(wd);
                }
                if let Some(cond) = cmd.condition.as_deref() {
                    values.push(cond);
                }
                values
            }

            Step::ExternalFile(ext) => {
                let mut values = vec![ext.uses.as_str()];
                if let Some(name) = ext.name.as_deref() {
                    values.push(name);
                }
                if let Some(server) = ext.server.as_deref() {
                    values.push(server);
                }
                values.extend(ext.with.values().map(|x| x.as_str()));
                values.extend(ext.env.values().map(|x| x.as_str()));
                values
            }

            Step::DownloadArtifact(download) => vec![download.to.as_str()],

            Step::UploadArtifact(upload) => vec![upload.upload.as_str()],
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Dependencies<'a> for Step {
    async fn local_deps(&'a self, fs: &FileSystem) -> Vec<Dependency<'a>> {
        if let Self::ExternalFile(external) = self
            && external.server.is_none()
            && matches!(fs.path(&external.uses).await.map(|x| x.is_yaml()), Ok(true))
        {
            return vec![Dependency::LocalFile(&external.uses)];
        }

        vec![]
    }

    async fn remote_deps(&'a self, manager: &PackageManager) -> Vec<Dependency<'a>> {
        if let Self::ExternalFile(external) = self {
            return vec![Dependency::Remote(Box::new(RemoteDependency::new(
                external.server.as_deref(),
                external.uses.as_str(),
                manager.is_package(&external.uses),
            )))];
        }
        vec![]
    }

    async fn jobs(&'a self) -> Vec<Dependency<'a>> {
        vec![]
    }

    async fn all(&'a self, manager: &PackageManager, fs: &FileSystem) -> Vec<Dependency<'a>> {
        let mut deps = self.local_deps(fs).await;
        deps.append(&mut self.remote_deps(manager).await);
        deps
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Step {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        _rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };

        let key = object.as_span().as_str();

        let value = match self {
            Self::ComplexSh(command) => match key {
                "name" => ExprValue::Text(ExprText::Ref(command.name.as_deref().unwrap_or(""))),
                "working_dir" => {
                    ExprValue::Text(ExprText::Ref(command.working_dir.as_deref().unwrap_or("")))
                }
                "run" => ExprValue::Text(ExprText::Ref(&command.run)),
                "outputs" => {
                    let Some(object) = path.next() else {
                        bail!("no output variable name provided");
                    };
                    let name = object.as_span().as_str();
                    wctx.get_output(OutputScope::Step, &command.id, name)?
                }
                value => bail!("invalid steps field: {value}"),
            },

            Self::ExternalFile(external) => match key {
                "outputs" => {
                    let Some(object) = path.next() else {
                        bail!("no output variable name provided");
                    };
                    let name = object.as_span().as_str();
                    wctx.get_output(OutputScope::Step, &external.id, name)?
                }
                value => bail!("invalid expression for step: {value}"),
            },

            Self::DownloadArtifact(_) => {
                bail!("invalid expression for step");
            }

            Self::UploadArtifact(_) => {
                bail!("invalid expression for step");
            }
        };

        Ok(value)
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Step {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            Step::ComplexSh(complex) => {
                debug!("Step is a complex shell command");
                ctx.push_section(&complex.id);

                if let Some(name) = complex.name.as_ref() {
                    debug!("Validating step's name value");
                    ctx.push_section("name");
                    ctx.validate_expressions(name, ExprScope::Runtime);
                    ctx.pop_section();
                }

                if let Some(wd) = complex.working_dir.as_ref() {
                    debug!("Validating step's working directory");
                    ctx.push_section("working_dir");
                    ctx.validate_expressions(wd, ExprScope::Runtime);
                    ctx.pop_section();
                }

                if let Some(condition) = complex.condition.as_ref() {
                    debug!("Validating step's if condition");
                    ctx.push_section("if");
                    let expr_count = ctx.expression_count(condition);
                    if expr_count == 0 {
                        ctx.append_error("Condition must contain exactly one expression");
                    } else if expr_count > 1 {
                        ctx.append_error("Condition must contain at most one expression");
                    } else {
                        ctx.validate_condition_expression(condition, ExprScope::Runtime);
                    }
                    ctx.pop_section();
                }

                debug!("Validating step's run command");
                ctx.push_section("run");
                if complex.run.trim().is_empty() {
                    ctx.append_error("Run command must not be empty");
                }
                ctx.validate_expressions(&complex.run, ExprScope::Runtime);
                ctx.pop_section();

                ctx.pop_section();
            }

            Step::ExternalFile(external) => {
                debug!("Step is an external file");
                ctx.push_section(&external.id);
                external.validate(ctx).await;
                ctx.pop_section();
            }

            Step::DownloadArtifact(download) => {
                debug!("Step is an artifact download");
                ctx.push_section(&download.id);
                download.validate(ctx).await;
                ctx.pop_section();
            }

            Step::UploadArtifact(upload) => {
                debug!("Step is an artifact upload");
                ctx.push_section(&upload.id);
                upload.validate(ctx).await;
                ctx.pop_section();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bld_config::BldConfig;
    use bld_core::fs::FileSystem;
    use bld_pkg::PackageManager;
    use bld_utils::sync::IntoArc;
    use mockall::predicate;
    use std::collections::HashMap;

    use crate::{
        action::v3::Action,
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext,
            exec::CommonExprExecutor,
            traits::{EvalExpr, ExprText, ExprValue, MockWritableRuntimeExprContext, OutputScope},
        },
        job::v3::Job,
        outputs::v3::Output,
        pipeline::v3::Pipeline,
        step::v3::{ShellCommand, Step},
        strategy::v3::{MatrixValue, Strategy},
        validator::v3::{CommonValidator, ConsumeValidator, ValidatorWritableRuntimeExprContext},
    };

    #[test]
    pub fn jobs_complex_step_expr_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                steps: vec![
                    Step::ComplexSh(Box::new(ShellCommand {
                        id: "second".to_string(),
                        name: Some("second_name".to_string()),
                        working_dir: Some("some_second_working_directory".to_string()),
                        run: "second_run_command".to_string(),
                        condition: Some("second_condition".to_string()),
                        strategy: None,
                    })),
                    Step::ComplexSh(Box::new(ShellCommand {
                        id: "third".to_string(),
                        name: Some("third_name".to_string()),
                        working_dir: Some("some_third_working_directory".to_string()),
                        run: "third_run_command".to_string(),
                        condition: Some("third_condition".to_string()),
                        strategy: None,
                    })),
                ],
                ..Default::default()
            },
        );
        pipeline.jobs.insert(
            "backup".to_string(),
            Job {
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "first".to_string(),
                    name: Some("first_name".to_string()),
                    working_dir: Some("some_first_working_directory".to_string()),
                    run: "first_run_command".to_string(),
                    condition: Some("first_condition".to_string()),
                    strategy: None,
                }))],
                ..Default::default()
            },
        );

        wctx.expect_get_exec_id().returning_st(|| Some("main"));
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let actual = exec.eval("${{ steps.second.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_second_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_third_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.condition }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.third.condition }}");
        assert!(actual.is_err());

        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning_st(|| Some("backup"));
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let actual = exec.eval("${{ steps.first.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_first_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.condition }}");
        assert!(actual.is_err());
    }

    #[test]
    pub fn steps_complex_step_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "second".to_string(),
            name: Some("second_name".to_string()),
            working_dir: Some("some_second_working_directory".to_string()),
            run: "second_run_command".to_string(),
            condition: Some("second_condition".to_string()),
            strategy: None,
        })));
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "third".to_string(),
            name: Some("third_name".to_string()),
            working_dir: Some("some_third_working_directory".to_string()),
            run: "third_run_command".to_string(),
            condition: Some("third_condition".to_string()),
            strategy: None,
        })));
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "first".to_string(),
            name: Some("first_name".to_string()),
            working_dir: Some("some_first_working_directory".to_string()),
            run: "first_run_command".to_string(),
            condition: Some("first_condition".to_string()),
            strategy: None,
        })));

        let exec = CommonExprExecutor::new(&action, &rctx, &wctx);

        let actual = exec.eval("${{ steps.second.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_second_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_third_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_first_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.condition }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.third.condition }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.first.condition }}");
        assert!(actual.is_err());
    }

    #[test]
    pub fn jobs_external_step_expr_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                steps: vec![
                    Step::ExternalFile(Box::default()),
                    Step::ExternalFile(Box::default()),
                ],
                ..Default::default()
            },
        );
        pipeline.jobs.insert(
            "backup".to_string(),
            Job {
                steps: vec![Step::ExternalFile(Box::default())],
                ..Default::default()
            },
        );

        wctx.expect_get_exec_id().returning_st(|| Some("main"));
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
        let actual = exec.eval("${{ jobs.main.first }}");
        assert!(actual.is_err());

        wctx.expect_get_exec_id().returning_st(|| Some("main"));
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
        let actual = exec.eval("${{ jobs.main.second }}");
        assert!(actual.is_err());

        wctx.expect_get_exec_id().returning_st(|| Some("backup"));
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
        let actual = exec.eval("${{ jobs.backup.third }}");
        assert!(actual.is_err());
    }

    #[test]
    pub fn steps_external_step_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut action = Action::default();
        action.steps.push(Step::ExternalFile(Box::default()));
        action.steps.push(Step::ExternalFile(Box::default()));
        action.steps.push(Step::ExternalFile(Box::default()));
        let exec = CommonExprExecutor::new(&action, &rctx, &wctx);

        let actual = exec.eval("${{ steps.main.first }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.main.second }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.backup.third }}");
        assert!(actual.is_err());
    }

    #[test]
    pub fn jobs_outputs_expr_eval_success() {
        // Arrange
        let data = [
            (
                "main",
                vec![
                    (
                        "build",
                        vec![
                            ("name", "john"),
                            ("surname", "doe"),
                            ("address", "some address"),
                        ],
                    ),
                    (
                        "test",
                        vec![
                            ("test_result", "success"),
                            ("tests_run", "3000"),
                            ("tests_skipped", "200"),
                        ],
                    ),
                ],
            ),
            (
                "second",
                vec![
                    ("format", vec![("lines", "30K")]),
                    ("lint", vec![("errors", "true")]),
                ],
            ),
        ];
        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();

        for (job, steps) in data.iter() {
            if !pipeline.jobs.contains_key(*job) {
                pipeline.jobs.insert(job.to_string(), Job::default());
            }

            for (step, outputs) in steps.iter() {
                let job_steps = &mut pipeline.jobs.get_mut(*job).unwrap().steps;
                if job_steps.iter().find(|x| x.is(step)).is_none() {
                    job_steps.push(Step::ComplexSh(Box::new(ShellCommand {
                        id: step.to_string(),
                        name: None,
                        run: String::new(),
                        condition: None,
                        working_dir: None,
                        strategy: None,
                    })));
                }

                wctx.expect_get_exec_id()
                    .times(outputs.len())
                    .returning(|| Some(job));

                for (name, value) in outputs.iter() {
                    wctx.expect_get_output()
                        .with(
                            predicate::eq(OutputScope::Step),
                            predicate::eq(*step),
                            predicate::eq(*name),
                        )
                        .times(1)
                        .returning(|_, _, _| Ok(ExprValue::Text(ExprText::Ref(value))));
                }
            }
        }

        for (_, steps) in data.iter() {
            for (step, outputs) in steps.iter() {
                for (name, value) in outputs.iter() {
                    let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
                    let expr = format!("{} steps.{step}.outputs.{name} {}", "${{", "}}");

                    // Act
                    let actual = exec.eval(&expr);

                    // Assert
                    assert!(matches!(
                        actual
                            .unwrap()
                            .try_eq(&ExprValue::Text(ExprText::Ref(value))),
                        Ok(ExprValue::Boolean(true))
                    ));
                }
            }
        }
    }

    #[test]
    pub fn action_outputs_expr_eval_success() {
        // Arrange
        let data = [
            (
                "build",
                vec![
                    ("name", "john"),
                    ("surname", "doe"),
                    ("address", "some address"),
                ],
            ),
            (
                "test",
                vec![
                    ("test_result", "success"),
                    ("tests_run", "3000"),
                    ("tests_skipped", "200"),
                ],
            ),
        ];
        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut action = Action::default();

        for (step, outputs) in data.iter() {
            if action.steps.iter().find(|x| x.is(step)).is_none() {
                action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
                    id: step.to_string(),
                    name: None,
                    run: String::new(),
                    condition: None,
                    working_dir: None,
                    strategy: None,
                })));
            }
            for (name, value) in outputs.iter() {
                wctx.expect_get_output()
                    .with(
                        predicate::eq(OutputScope::Step),
                        predicate::eq(*step),
                        predicate::eq(*name),
                    )
                    .times(1)
                    .returning(|_, _, _| Ok(ExprValue::Text(ExprText::Ref(value))));
            }
        }

        for (step, outputs) in data.iter() {
            for (name, value) in outputs.iter() {
                let exec = CommonExprExecutor::new(&action, &rctx, &wctx);
                let expr = format!("{} steps.{step}.outputs.{name} {}", "${{", "}}");

                // Act
                let actual = exec.eval(&expr);

                // Assert
                assert!(matches!(
                    actual
                        .unwrap()
                        .try_eq(&ExprValue::Text(ExprText::Ref(value))),
                    Ok(ExprValue::Boolean(true))
                ));
            }
        }
    }

    async fn validate_action(action: &Action) -> anyhow::Result<()> {
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default();
        let expr_wctx = vec![ValidatorWritableRuntimeExprContext::new("action")];

        CommonValidator::new(action, config, fs, package_manager, &expr_rctx, &expr_wctx)?
            .validate()
            .await
    }

    #[tokio::test]
    pub async fn condition_without_expression_wrapper_fails_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "first".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: Some("true".to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    pub async fn condition_with_multiple_expressions_fails_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "first".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: Some("${{ true }} ${{ false }}".to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    pub async fn condition_with_single_expression_passes_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "first".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: Some("${{ true }}".to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn condition_with_step_output_text_comparison_passes_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            working_dir: None,
            run: "echo \"value=ok\" >> $BLD_OUTPUTS".to_string(),
            condition: None,
            strategy: None,
        })));
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "after".to_string(),
            name: None,
            working_dir: None,
            run: "echo done".to_string(),
            condition: Some(r#"${{ steps.build.outputs.value == "ok" }}"#.to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn condition_with_step_output_number_comparison_passes_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            working_dir: None,
            run: "echo \"count=5\" >> $BLD_OUTPUTS".to_string(),
            condition: None,
            strategy: None,
        })));
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "after".to_string(),
            name: None,
            working_dir: None,
            run: "echo done".to_string(),
            condition: Some("${{ steps.build.outputs.count > 3 }}".to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn condition_with_text_true_or_false_passes_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "first".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: Some("${{ \"true\" }}".to_string()),
            strategy: None,
        })));
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "second".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: Some("${{ \"false\" }}".to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn condition_with_number_fails_validation_with_type_in_message() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "first".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: Some("${{ 1 }}".to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        let err = result.unwrap_err();
        assert!(err.to_string().contains("number"));
    }

    #[tokio::test]
    pub async fn condition_with_array_fails_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "first".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: Some("${{ [1, 2] }}".to_string()),
            strategy: None,
        })));

        let result = validate_action(&action).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    pub async fn strategy_matrix_from_step_output_passes_validation() {
        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Expr("${{ steps.build.outputs.oses }}".to_string()),
        );

        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            working_dir: None,
            run: "echo \"oses=[linux, windows]\" >> $BLD_OUTPUTS".to_string(),
            condition: None,
            strategy: None,
        })));
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "after".to_string(),
            name: None,
            working_dir: None,
            run: "echo ${{ matrix.os }}".to_string(),
            condition: None,
            strategy: Some(Strategy {
                matrix,
                fail_fast: None,
            }),
        })));

        let result = validate_action(&action).await;

        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[tokio::test]
    pub async fn output_reading_input_and_declared_step_output_passes_validation() {
        let mut action = Action::default();
        action.inputs.insert(
            "tag".to_string(),
            crate::inputs::v3::Input::Simple(String::new()),
        );
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            working_dir: None,
            run: "echo \"digest=abc\" >> $BLD_OUTPUTS".to_string(),
            condition: None,
            strategy: None,
        })));
        action.outputs.insert(
            "image".to_string(),
            Output::Simple("${{ inputs.tag }}".to_string()),
        );
        action.outputs.insert(
            "digest".to_string(),
            Output::Complex {
                description: Some("The digest of the built image".to_string()),
                value: "${{ steps.build.outputs.digest }}".to_string(),
            },
        );

        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let mut inputs = HashMap::new();
        inputs.insert("tag".to_string(), String::new());
        let expr_rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        };
        let expr_wctx = vec![ValidatorWritableRuntimeExprContext::new("action")];

        let result =
            CommonValidator::new(&action, config, fs, package_manager, &expr_rctx, &expr_wctx)
                .unwrap()
                .validate()
                .await;

        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[tokio::test]
    pub async fn output_naming_undeclared_step_fails_validation() {
        let mut action = Action::default();
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            working_dir: None,
            run: "echo hello".to_string(),
            condition: None,
            strategy: None,
        })));
        action.outputs.insert(
            "digest".to_string(),
            Output::Simple("${{ steps.missing.outputs.digest }}".to_string()),
        );

        let result = validate_action(&action).await;

        assert!(result.is_err());
    }
}
