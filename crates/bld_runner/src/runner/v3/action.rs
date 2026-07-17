use std::{collections::HashMap, fmt::Write, sync::Arc};

use anyhow::{Result, anyhow, bail};
use bld_config::BldConfig;
use bld_core::{
    artifacts::Artifacts, context::Context, fs::FileSystem, logger::Logger, platform::Platform,
    regex::RegexCache,
};
use bld_models::dtos::ExecClientMessage;
use bld_pkg::PackageManager;
use bld_sock::ExecClient;
use bld_utils::sync::IntoArc;
use regex::Regex;
use tracing::debug;

use crate::{
    RunnerBuilder,
    action::v3::Action,
    artifacts::v3::{DownloadArtifact, UploadArtifact},
    expr::v3::{
        context::CommonReadonlyRuntimeExprContext,
        exec::CommonExprExecutor,
        traits::{EvalExpr, ExprValue},
    },
    external::v3::External,
    runner::v3::state::{ActionState, RootState, State},
    step::v3::{ShellCommand, Step},
};

use super::common::RecursiveFuture;

pub struct ActionRunner<S: RootState> {
    pub logger: Arc<Logger>,
    pub action: Action,
    pub platform: Arc<Platform>,
    pub artifacts: Arc<Artifacts>,
    pub expr_regex: Regex,
    pub expr_rctx: CommonReadonlyRuntimeExprContext,
    pub state: S,
    pub config: Arc<BldConfig>,
    pub fs: Arc<FileSystem>,
    pub run_ctx: Arc<Context>,
    pub regex_cache: Arc<RegexCache>,
    pub package_manager: Arc<PackageManager>,
}

impl<S: RootState> ActionRunner<S> {
    async fn info(&self) -> Result<()> {
        debug!("printing action informantion");

        let mut message = String::new();

        writeln!(message, "{:<15}: {}", "Name", self.action.name)?;
        writeln!(message, "{:<15}: 3", "Version")?;

        self.logger.write_line(message).await
    }

    fn eval_all_expr(&mut self, value: &str) -> Result<String> {
        let expr_exec = CommonExprExecutor::new(&self.action, &self.expr_rctx, &self.state);

        let mut result = value.to_string();
        for entry in self.expr_regex.find_iter(value) {
            let entry = entry.as_str();
            let expr_value = expr_exec.eval(entry)?.to_string();
            result = result.replace(entry, &expr_value);
        }

        Ok(result)
    }

    fn condition(&mut self, condition: Option<&str>) -> Result<bool> {
        let Some(condition) = condition else {
            return Ok(true);
        };

        debug!("evaluating condition {condition} for step");

        let matches = self.expr_regex.find_iter(condition);

        if matches.count() > 1 {
            bail!("more than one condition found for step");
        };

        let expr_exec = CommonExprExecutor::new(&self.action, &self.expr_rctx, &self.state);
        let value = expr_exec.eval(condition)?;
        Ok(matches!(value, ExprValue::Boolean(true)))
    }

    async fn shell(
        &mut self,
        step_id: &str,
        working_dir: &Option<String>,
        command: &str,
    ) -> Result<()> {
        debug!("start execution of exec section for step");
        debug!("executing shell command {}", command);

        let cmd = self.eval_all_expr(command)?;
        let outputs = self
            .platform
            .shell(self.logger.clone(), working_dir, &cmd)
            .await?;

        self.state.set_outputs(step_id, outputs)?;

        Ok(())
    }

    async fn steps(&mut self) -> Result<()> {
        debug!("starting execution of action steps");
        let action = self.action.clone();
        for step in &action.steps {
            self.run_step(step, &HashMap::new()).await?;
        }
        Ok(())
    }

    async fn run_step(&mut self, step: &Step, job_matrix: &HashMap<String, String>) -> Result<()> {
        let Some(strategy) = step.strategy() else {
            self.state.set_matrix(job_matrix.clone());
            return self.dispatch_step(step).await;
        };

        let exec = CommonExprExecutor::new(&self.action, &self.expr_rctx, &self.state);
        let combinations = strategy.combinations(&exec)?;
        let fail_fast = strategy.resolve_fail_fast(&exec)?;

        let mut errors: Vec<String> = Vec::new();
        for combination in combinations {
            let mut merged = job_matrix.clone();
            merged.extend(combination);
            self.state.set_matrix(merged);
            if let Err(e) = self.dispatch_step(step).await {
                if fail_fast {
                    return Err(e);
                }
                errors.push(e.to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(errors.join("\n")))
        }
    }

    async fn dispatch_step(&mut self, step: &Step) -> Result<()> {
        self.state.update_node_state(step.id(), State::Running);
        match step {
            Step::ComplexSh(complex) => self.complex_shell(complex).await,
            Step::ExternalFile(external) => self.external(external).await,
            Step::DownloadArtifact(download) => self.download_artifact(download).await,
            Step::UploadArtifact(upload) => self.upload_artifact(upload).await,
        }
        .inspect(|_| self.state.update_node_state(step.id(), State::Completed))
        .inspect_err(|e| {
            self.state.update_node_state(
                step.id(),
                State::Failed {
                    error: e.to_string(),
                },
            )
        })
    }

    async fn complex_shell(&mut self, complex: &ShellCommand) -> Result<()> {
        let condition = complex.condition.as_deref();

        if !self.condition(condition)? {
            debug!("condition failed, skiping step");
            return Ok(());
        }

        if let Some(name) = complex.name.as_ref() {
            let mut message = String::new();
            writeln!(message, "{:<15}: {name}", "Step")?;
            self.logger.write_line(message).await?;
        }
        self.shell(&complex.id, &complex.working_dir, &complex.run)
            .await?;
        Ok(())
    }

    async fn external(&mut self, external: &External) -> Result<()> {
        if let Some(name) = external.name.as_ref() {
            let mut message = String::new();
            writeln!(message, "{:<15}: {name}", "Step")?;
            self.logger.write_line(message).await?;
        }

        debug!("calling external pipeline or action {}", external.uses);

        match external.server.as_ref() {
            Some(server) => self.server_external(server, external).await?,
            None => self.local_external(external).await?,
        };

        Ok(())
    }

    async fn local_external(&mut self, details: &External) -> Result<()> {
        debug!("building runner for child file");

        let inputs = self.variables_external(&details.with)?;
        let env = self.variables_external(&details.env)?;

        let runner = RunnerBuilder::default()
            .run_id(&self.expr_rctx.run_id)
            .run_start_time(&self.expr_rctx.run_start_time)
            .config(self.config.clone())
            .fs(self.fs.clone())
            .file(&details.uses)
            .logger(self.logger.clone())
            .env(env.into_arc())
            .inputs(inputs.into_arc())
            .context(self.run_ctx.clone())
            .platform(self.platform.clone())
            .regex_cache(self.regex_cache.clone())
            .package_manager(self.package_manager.clone())
            .artifacts(self.artifacts.clone())
            .is_child(true)
            .build()
            .await?;

        debug!("starting child file runner");
        runner.run().await?;

        Ok(())
    }

    async fn server_external(&mut self, server: &str, details: &External) -> Result<()> {
        let inputs = self.variables_external(&details.with)?;
        let env = self.variables_external(&details.env)?;

        debug!("establishing web socket connection with server {}", server);

        let client = ExecClient::connect(
            self.config.clone(),
            server.to_owned(),
            self.logger.clone(),
            self.run_ctx.clone(),
        )
        .await?;

        debug!("sending message for pipeline execution over the web socket");

        client
            .run(ExecClientMessage::EnqueueRun {
                name: details.uses.to_owned(),
                env: Some(env),
                inputs: Some(inputs),
            })
            .await
    }

    fn variables_external(
        &mut self,
        vars: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>> {
        let mut result = HashMap::new();
        for (name, value) in vars {
            let value = self.eval_all_expr(value)?;
            result.insert(name.to_string(), value);
        }
        Ok(result)
    }

    async fn download_artifact(&mut self, download: &DownloadArtifact) -> Result<()> {
        let local_path = self.eval_all_expr(&download.to)?;
        self.artifacts
            .download(self.platform.clone(), &download.download, &local_path)
            .await
    }

    async fn upload_artifact(&mut self, upload: &UploadArtifact) -> Result<()> {
        let local_path = self.eval_all_expr(&upload.upload)?;
        self.artifacts
            .upload(self.platform.clone(), &upload.name, &local_path)
            .await
    }

    async fn execute(mut self) -> Result<()> {
        self.state.update_state(State::Running);
        self.info().await.inspect_err(|e| {
            self.state.update_state(State::Failed {
                error: e.to_string(),
            })
        })?;
        self.steps().await.inspect_err(|e| {
            self.state.update_state(State::Failed {
                error: e.to_string(),
            })
        })?;
        self.state.update_state(State::Completed);
        Ok(())
    }
}

impl ActionRunner<ActionState> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        logger: Arc<Logger>,
        action: Action,
        platform: Arc<Platform>,
        artifacts: Arc<Artifacts>,
        expr_regex: Regex,
        expr_rctx: CommonReadonlyRuntimeExprContext,
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        run_ctx: Arc<Context>,
        regex_cache: Arc<RegexCache>,
        package_manager: Arc<PackageManager>,
    ) -> Self {
        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
        }
        Self {
            logger,
            action,
            platform,
            artifacts,
            expr_regex,
            expr_rctx,
            state,
            config,
            fs,
            run_ctx,
            regex_cache,
            package_manager,
        }
    }

    pub fn run(self) -> RecursiveFuture {
        Box::pin(async move { self.execute().await })
    }
}

#[cfg(test)]
mod tests {
    use bld_config::BldConfig;
    use bld_core::{
        artifacts::Artifacts, context::Context, fs::FileSystem, logger::Logger, platform::Platform,
        regex::RegexCache,
    };
    use bld_pkg::PackageManager;
    use bld_utils::sync::IntoArc;
    use regex::Regex;

    use std::collections::HashMap;

    use crate::{
        action::v3::Action,
        expr::v3::{context::CommonReadonlyRuntimeExprContext, parser::EXPR_REGEX},
        runner::v3::{ActionRunner, RootState, State, state::MockRootState},
        step::v3::{ShellCommand, Step},
        strategy::v3::{FailFastValue, MatrixValue, Strategy},
    };

    #[test]
    pub fn condition_eval_success() {
        let logger = Logger::mock().into_arc();
        let action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let state = MockRootState::new();
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut runner = ActionRunner {
            logger,
            action,
            platform,
            artifacts,
            expr_regex: regex,
            expr_rctx: rctx,
            state,
            config,
            fs,
            run_ctx,
            regex_cache,
            package_manager,
        };

        assert!(matches!(runner.condition(None), Ok(true)));

        assert!(matches!(runner.condition(Some("${{ true }}")), Ok(true)));

        assert!(matches!(
            runner.condition(Some("${{ \"John\" == \"James\" }}")),
            Ok(false)
        ));

        assert!(runner.condition(Some("${{ true == \"James\" }}")).is_err());

        assert!(
            runner
                .condition(Some("hello world ${{ true == \"James\" }}"))
                .is_err()
        );
    }

    #[tokio::test]
    pub async fn run_state_management_success() {
        // Arrange
        let data: Vec<String> = (0..10).map(|x| format!("id-{x}")).collect();
        let logger = Logger::mock().into_arc();
        let mut action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut state = MockRootState::new();
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        state.expect_set_matrix().returning(|_| ());

        for node_id in data {
            state
                .expect_update_state()
                .withf(|state| matches!(state, State::Running))
                .return_once(|_| ());

            let node_id_arg = node_id.clone();
            state
                .expect_update_node_state()
                .withf(move |node_id, state| {
                    node_id == node_id_arg && matches!(state, State::Running)
                })
                .return_once(|_, _| ());

            state.expect_set_outputs().returning_st(|_, _| Ok(()));

            let node_id_arg = node_id.clone();
            state
                .expect_update_node_state()
                .withf(move |node_id, state| {
                    node_id == node_id_arg && matches!(state, State::Completed)
                })
                .return_once(|_, _| ());

            action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
                id: node_id.clone(),
                name: None,
                run: "echo hello".to_string(),
                condition: None,
                working_dir: None,
                strategy: None,
            })));

            state
                .expect_update_state()
                .withf(|state| matches!(state, State::Completed))
                .return_once(|_| ());
        }

        let runner = ActionRunner {
            logger,
            action,
            platform,
            artifacts,
            expr_regex: regex,
            expr_rctx: rctx,
            state,
            config,
            fs,
            run_ctx,
            regex_cache,
            package_manager,
        };

        // Act
        runner.execute().await.unwrap();
    }

    #[tokio::test]
    pub async fn step_level_matrix_expansion_runs_all_combinations_success() {
        let logger = Logger::mock().into_arc();
        let mut action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut state = crate::runner::v3::state::ActionState::default();
        state.add_node("build");
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Array(vec!["linux".to_string(), "windows".to_string()]),
        );

        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            run: "echo ${{ matrix.os }}".to_string(),
            condition: None,
            working_dir: None,
            strategy: Some(Strategy {
                matrix,
                fail_fast: None,
            }),
        })));

        let runner = ActionRunner {
            logger,
            action,
            platform,
            artifacts,
            expr_regex: regex,
            expr_rctx: rctx,
            state,
            config,
            fs,
            run_ctx,
            regex_cache,
            package_manager,
        };

        let result = runner.execute().await;
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[tokio::test]
    pub async fn step_matrix_fail_fast_true_stops_at_first_failure() {
        let logger = Logger::mock().into_arc();
        let mut action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut state = MockRootState::new();
        state.expect_update_state().returning(|_| ());
        state.expect_set_matrix().returning(|_| ());
        state
            .expect_update_node_state()
            .withf(|_, state| matches!(state, State::Running))
            .times(1)
            .returning(|_, _| ());
        state
            .expect_update_node_state()
            .withf(|_, state| matches!(state, State::Failed { .. }))
            .times(1)
            .returning(|_, _| ());

        let mut matrix = HashMap::new();
        matrix.insert(
            "n".to_string(),
            MatrixValue::Array(vec!["1".to_string(), "2".to_string(), "3".to_string()]),
        );

        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            run: "echo hello".to_string(),
            condition: Some("${{ 1 }} ${{ 2 }}".to_string()),
            working_dir: None,
            strategy: Some(Strategy {
                matrix,
                fail_fast: None,
            }),
        })));

        let runner = ActionRunner {
            logger,
            action,
            platform,
            artifacts,
            expr_regex: regex,
            expr_rctx: rctx,
            state,
            config,
            fs,
            run_ctx,
            regex_cache,
            package_manager,
        };

        let result = runner.execute().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    pub async fn step_matrix_fail_fast_false_runs_all_and_aggregates() {
        let logger = Logger::mock().into_arc();
        let mut action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut state = MockRootState::new();
        state.expect_update_state().returning(|_| ());
        state.expect_set_matrix().returning(|_| ());
        state
            .expect_update_node_state()
            .withf(|_, state| matches!(state, State::Running))
            .times(3)
            .returning(|_, _| ());
        state
            .expect_update_node_state()
            .withf(|_, state| matches!(state, State::Failed { .. }))
            .times(3)
            .returning(|_, _| ());

        let mut matrix = HashMap::new();
        matrix.insert(
            "n".to_string(),
            MatrixValue::Array(vec!["1".to_string(), "2".to_string(), "3".to_string()]),
        );

        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            run: "echo hello".to_string(),
            condition: Some("${{ 1 }} ${{ 2 }}".to_string()),
            working_dir: None,
            strategy: Some(Strategy {
                matrix,
                fail_fast: Some(FailFastValue::Bool(false)),
            }),
        })));

        let runner = ActionRunner {
            logger,
            action,
            platform,
            artifacts,
            expr_regex: regex,
            expr_rctx: rctx,
            state,
            config,
            fs,
            run_ctx,
            regex_cache,
            package_manager,
        };

        let result = runner.execute().await;
        assert!(result.is_err());
    }
}
