use crate::{
    inputs::v3::Input,
    job::v3::Job,
    traits::{IntoVariables, Variables},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(feature = "all")]
use {
    crate::{
        deps::v3::{Dependencies, Dependency},
        expr::v3::{
            context::out_of_scope,
            parser::Rule,
            traits::{
                EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext,
                WritableRuntimeExprContext,
            },
        },
        validator::v3::{ExprScope, Validate, ValidatorContext},
    },
    anyhow::{Result, bail},
    bld_config::definitions::{
        KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
        KEYWORD_RUN_PROPS_START_TIME_V3,
    },
    bld_core::fs::FileSystem,
    bld_pkg::PackageManager,
    cron::Schedule,
    pest::iterators::Pairs,
    std::{iter::Peekable, str::FromStr},
    tracing::debug,
};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,

    pub cron: Option<String>,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub inputs: HashMap<String, Input>,

    #[serde(default)]
    pub jobs: HashMap<String, Job>,
}

impl Pipeline {
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
        ctx.push_section("cron");
        if ctx.contains_expressions(cron) {
            ctx.validate_expressions(cron, ExprScope::StartOfRun);
        } else if let Err(e) = Schedule::from_str(cron) {
            let error = format!("{cron} {e}");
            ctx.append_error(&error);
        }
        ctx.pop_section();
    }

    #[cfg(feature = "all")]
    async fn validate_jobs<'a, C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        ctx.push_section("jobs");

        if self.jobs.is_empty() {
            ctx.append_error("Pipeline must have at least one job defined");
        }

        let mut job_ids = HashSet::new();
        let mut needs_defined = true;
        for (name, job) in &self.jobs {
            ctx.push_job_section(name);
            debug!("Validating {name} job's steps");
            let job_id = &job.id;
            if !job_ids.insert(job_id) {
                ctx.push_section(job_id);
                ctx.append_error(&format!("Duplicate job id '{job_id}' found"));
                ctx.pop_section();
            }

            debug!("Validating {name} job's needs section");
            ctx.push_section("needs");
            for need in job.needs_iter() {
                if !self.jobs.contains_key(need) {
                    needs_defined = false;
                    ctx.append_error(&format!("job depends on undefined job '{need}'"));
                }
            }
            ctx.pop_section();

            job.validate(ctx).await;
            ctx.pop_section();
        }

        if needs_defined {
            debug!("Validating pipeline's jobs dependency graph for cycles");
            if let Err(e) = crate::dag::Dag::try_from(self) {
                ctx.append_error(&e.to_string());
            }
        }

        ctx.pop_section();
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
impl<'a> Dependencies<'a> for Pipeline {
    async fn local_deps(&'a self, fs: &FileSystem) -> Vec<Dependency<'a>> {
        let mut dependecies = vec![];
        for job in self.jobs.values() {
            dependecies.append(&mut job.local_deps(fs).await);
        }
        dependecies
    }

    async fn remote_deps(&'a self, manager: &PackageManager) -> Vec<Dependency<'a>> {
        let mut dependecies = vec![];
        for job in self.jobs.values() {
            dependecies.append(&mut job.remote_deps(manager).await);
        }
        dependecies
    }

    async fn jobs(&'a self) -> Vec<Dependency<'a>> {
        let mut set = HashSet::new();
        for name in self.jobs.keys() {
            set.insert(name.as_str());
        }
        set.into_iter().map(Dependency::Job).collect()
    }

    async fn all(&'a self, manager: &PackageManager, fs: &FileSystem) -> Vec<Dependency<'a>> {
        let mut deps = self.local_deps(fs).await;
        deps.append(&mut self.remote_deps(manager).await);
        deps
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Pipeline {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };

        let object_path = object.as_span().as_str();
        let mut object_parts = object.into_inner().peekable();
        let Some(part) = object_parts.peek() else {
            bail!("expected at least one part in the object path");
        };

        match part.as_span().as_str() {
            "name" => {
                let name = self.name.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(name)))
            }

            "cron" => {
                let cron = self.cron.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(cron)))
            }

            "inputs" => {
                let Some(part) = object_parts.nth(1) else {
                    bail!("expected name of input in object path");
                };
                let name = part.as_span().as_str();
                rctx.get_input(name)
                    .map(|x| ExprValue::Text(ExprText::Ref(x)))
            }

            "env" => {
                let Some(part) = object_parts.nth(1) else {
                    bail!("expected name of env variable in object path");
                };
                let name = part.as_span().as_str();
                rctx.get_env(name)
                    .map(|x| ExprValue::Text(ExprText::Ref(x)))
            }

            // Keywords section
            value if value == KEYWORD_BLD_DIR_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_root_dir())))
            }

            value if value == KEYWORD_PROJECT_DIR_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_project_dir())))
            }

            value if value == KEYWORD_RUN_PROPS_ID_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_run_id())))
            }

            value if value == KEYWORD_RUN_PROPS_START_TIME_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_run_start_time())))
            }

            // Move evaluation to the job level
            _ => {
                let Some(exec_id) = wctx.get_exec_id() else {
                    return Err(out_of_scope(object_path));
                };
                let Some(job) = self.jobs.get(exec_id) else {
                    bail!("unable to find executing job id");
                };
                job.eval_object(&mut object_parts, rctx, wctx)
            }
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Pipeline {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating pipeline");

        if let Some(name) = self.name.as_ref() {
            debug!("Validating pipeline's name value");
            ctx.push_section("name");
            ctx.validate_expressions(name, ExprScope::StartOfRun);
            ctx.pop_section();
        }

        debug!("Validating pipeline's cron value");
        self.validate_cron(ctx);

        debug!("Validating pipeline's inputs section");
        ctx.push_section("inputs");
        for (name, input) in self.inputs.iter() {
            debug!("Validating input: {}", name);
            ctx.push_section(name);
            input.validate(ctx).await;
            ctx.pop_section();
        }
        ctx.pop_section();

        debug!("Validating pipeline's env section");
        ctx.push_section("env");
        ctx.validate_env(&self.env, ExprScope::StartOfRun);
        ctx.pop_section();

        debug!("Validating that the pipeline's inputs and env can be resolved");
        ctx.validate_start_of_run_values(&self.inputs, &self.env);

        debug!("Validating pipeline's jobs section");
        self.validate_jobs(ctx).await;
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use bld_config::BldConfig;
    use bld_core::fs::FileSystem;
    use bld_pkg::PackageManager;
    use bld_utils::sync::IntoArc;

    use crate::{
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext,
            exec::CommonExprExecutor,
            traits::{EvalExpr, ExprText, ExprValue, MockWritableRuntimeExprContext},
        },
        inputs::v3::Input,
        job::v3::{Job, Needs},
        step::v3::{ShellCommand, Step},
        validator::v3::{ExprScope, RunnerFileValidator, Validate, ValidatorContext},
    };
    use crate::{files::v3::RunnerFile, validator::v3::ConsumeValidator};

    use super::Pipeline;

    struct RecordingValidatorContext {
        errors: Vec<String>,
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
    }

    impl RecordingValidatorContext {
        fn new() -> Self {
            let config = BldConfig::default().into_arc();
            Self {
                errors: Vec::new(),
                fs: FileSystem::local(config.clone()).into_arc(),
                package_manager: PackageManager::new(config.clone()).into_arc(),
                config,
            }
        }
    }

    impl<'a> ValidatorContext<'a> for RecordingValidatorContext {
        fn get_config(&self) -> Arc<BldConfig> {
            self.config.clone()
        }

        fn get_fs(&self) -> Arc<FileSystem> {
            self.fs.clone()
        }

        fn get_package_manager(&self) -> Arc<PackageManager> {
            self.package_manager.clone()
        }

        fn push_section(&mut self, _section: &'a str) {}

        fn push_job_section(&mut self, _section: &'a str) {}

        fn pop_section(&mut self) {}

        fn clear_section(&mut self) {}

        fn append_error(&mut self, error: &str) {
            self.errors.push(error.to_string());
        }

        fn expression_count(&self, _value: &str) -> usize {
            0
        }

        fn contains_expressions(&mut self, _value: &str) -> bool {
            false
        }

        fn validate_expressions(&mut self, _symbol: &'a str, _scope: ExprScope) {}

        fn validate_file_path(&mut self, _value: &'a str) {}

        fn validate_env(&mut self, _env: &'a HashMap<String, String>, _scope: ExprScope) {}

        fn validate_array_expression(&mut self, _symbol: &'a str, _scope: ExprScope) {}

        fn matrix_refs(&self, _value: &str) -> Vec<String> {
            vec![]
        }

        fn validate_start_of_run_values(
            &mut self,
            _inputs: &'a HashMap<String, Input>,
            _env: &'a HashMap<String, String>,
        ) {
        }
    }

    fn job_with_needs(needs: Option<Needs>) -> Job {
        Job {
            needs,
            steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                run: "echo hello".to_string(),
                ..Default::default()
            }))],
            ..Default::default()
        }
    }

    fn complex_input(default: &str) -> Input {
        Input::Complex {
            description: None,
            default: Some(default.to_string()),
            required: false,
        }
    }

    fn with_single_job(mut pipeline: Pipeline) -> Pipeline {
        if !pipeline.jobs.is_empty() {
            return pipeline;
        }
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo hello".to_string(),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );
        pipeline
    }

    async fn validate_pipeline(pipeline: Pipeline) -> anyhow::Result<()> {
        let config = BldConfig::default().into_arc();
        let file_system = FileSystem::local(config.clone()).into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let file = RunnerFile::PipelineFileType(Box::new(with_single_job(pipeline)));

        RunnerFileValidator::new(&file, config, file_system, package_manager)?
            .validate()
            .await
    }

    #[tokio::test]
    pub async fn start_of_run_values_validation_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "worktree_root".to_string(),
            complex_input("${{ bld_project_dir }}/../worktrees"),
        );
        pipeline.env.insert(
            "LOGS".to_string(),
            "${{ inputs.worktree_root }}/logs".to_string(),
        );

        let result = validate_pipeline(pipeline).await;
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    /// A required input has no value until a run supplies one, so validation stands in an
    /// empty value rather than reporting every use of it as an error.
    #[tokio::test]
    pub async fn required_input_without_default_validation_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "worktree_dir".to_string(),
            Input::Complex {
                description: None,
                default: None,
                required: true,
            },
        );
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo ${{ inputs.worktree_dir }}".to_string(),
                    working_dir: Some("${{ inputs.worktree_dir }}".to_string()),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );

        let result = validate_pipeline(pipeline).await;
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[tokio::test]
    pub async fn input_default_referencing_another_default_validation_failure() {
        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("first".to_string(), complex_input("/root"));
        pipeline.inputs.insert(
            "second".to_string(),
            complex_input("${{ inputs.first }}/sub"),
        );

        let Err(e) = validate_pipeline(pipeline).await else {
            panic!("expected a validation error for a default referencing another default");
        };
        assert!(
            e.to_string().contains("unable to resolve inputs.second"),
            "{e}"
        );
    }

    #[tokio::test]
    pub async fn runtime_expr_in_input_default_and_env_validation_failure() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "image".to_string(),
            complex_input("${{ steps.build.outputs.image }}"),
        );
        pipeline
            .env
            .insert("OS".to_string(), "${{ matrix.os }}".to_string());

        let Err(e) = validate_pipeline(pipeline).await else {
            panic!("expected a validation error for runtime expressions");
        };
        let error = e.to_string();
        assert!(
            error.contains(
                "[inputs > image > default] 'steps.build.outputs.image' is not available at the start of a run"
            ),
            "{error}"
        );
        assert!(
            error.contains("[env > OS] 'matrix.os' is not available at the start of a run"),
            "{error}"
        );
    }

    #[test]
    pub fn name_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        let data = vec![Some("test"), Some("hello world"), Some(""), None];

        for entry in data {
            pipeline.name = entry.map(|x| x.to_string());

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
            let Ok(value) = exec.eval("${{ name }}") else {
                panic!("result is an error during expression evaluation");
            };

            let expected = entry
                .map(|x| ExprValue::Text(ExprText::Ref(x)))
                .unwrap_or_else(|| ExprValue::Text(ExprText::Ref("")));

            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn cron_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        let data = vec![
            Some("30 * * * * 1"),
            Some("H 5 * * 1"),
            Some("1 M * * * 2"),
            None,
        ];

        for entry in data {
            pipeline.cron = entry.map(|x| x.to_string());

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
            let Ok(value) = exec.eval("${{ cron }}") else {
                panic!("result is an error during expression evaluation");
            };

            let expected = entry
                .map(|x| ExprValue::Text(ExprText::Ref(x)))
                .unwrap_or_else(|| ExprValue::Text(ExprText::Ref("")));

            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn matrix_expr_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert("test".to_string(), Job::default());

        wctx.expect_get_exec_id().returning(|| Some("test"));

        wctx.expect_get_matrix_value()
            .with(mockall::predicate::eq("os"))
            .returning(|_| Ok("linux"));

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
        let actual = exec.eval("${{ matrix.os }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("linux"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn matrix_undefined_expr_eval_failure() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert("test".to_string(), Job::default());

        wctx.expect_get_exec_id().returning(|| Some("test"));

        wctx.expect_get_matrix_value()
            .with(mockall::predicate::eq("missing"))
            .returning(|name| Err(anyhow::anyhow!("matrix value '{name}' not found")));

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
        assert!(exec.eval("${{ matrix.missing }}").is_err());
    }

    #[test]
    pub fn inputs_use_rctx_expr_eval_success() {
        // Arrange
        let data: HashMap<String, String> = vec![
            ("name", "john"),
            ("surname", "doe"),
            ("age", "30"),
            ("address", "some address"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext {
            inputs: data.clone().into_arc(),
            ..Default::default()
        };
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (name, expected) in data {
            let expr = format!("{} inputs.{name} {}", "${{", "}}");
            // Act
            let actual = exec.eval(&expr).unwrap();

            // Assert
            assert!(matches!(
                actual.try_eq(&ExprValue::Text(ExprText::Ref(&expected))),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn env_use_rctx_expr_eval_success() {
        // Arrange
        let data: HashMap<String, String> = vec![
            ("name", "john"),
            ("surname", "doe"),
            ("age", "30"),
            ("address", "some address"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext {
            env: data.clone().into_arc(),
            ..Default::default()
        };
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (name, expected) in data {
            let expr = format!("{} env.{name} {}", "${{", "}}");
            // Act
            let actual = exec.eval(&expr).unwrap();

            // Assert
            assert!(matches!(
                actual.try_eq(&ExprValue::Text(ExprText::Ref(&expected))),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[tokio::test]
    pub async fn validate_rejects_job_depending_on_undefined_job() {
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "b".to_string(),
            job_with_needs(Some(Needs::Single("missing".to_string()))),
        );

        let mut ctx = RecordingValidatorContext::new();
        pipeline.validate(&mut ctx).await;

        assert!(
            ctx.errors
                .iter()
                .any(|e| e.contains("depends on undefined job") && e.contains("missing")),
            "expected an undefined job dependency error, got: {:?}",
            ctx.errors
        );
    }

    #[tokio::test]
    pub async fn validate_rejects_cyclic_job_dependencies() {
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "a".to_string(),
            job_with_needs(Some(Needs::Single("b".to_string()))),
        );
        pipeline.jobs.insert(
            "b".to_string(),
            job_with_needs(Some(Needs::Single("a".to_string()))),
        );

        let mut ctx = RecordingValidatorContext::new();
        pipeline.validate(&mut ctx).await;

        assert!(
            ctx.errors.iter().any(|e| e.contains("cyclic dependency")),
            "expected a cyclic dependency error, got: {:?}",
            ctx.errors
        );
    }

    #[tokio::test]
    pub async fn validate_accepts_valid_job_dependencies() {
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert("a".to_string(), job_with_needs(None));
        pipeline.jobs.insert(
            "b".to_string(),
            job_with_needs(Some(Needs::Single("a".to_string()))),
        );

        let mut ctx = RecordingValidatorContext::new();
        pipeline.validate(&mut ctx).await;

        assert!(
            !ctx.errors
                .iter()
                .any(|e| e.contains("undefined job") || e.contains("cyclic dependency")),
            "did not expect dependency errors, got: {:?}",
            ctx.errors
        );
    }
}
