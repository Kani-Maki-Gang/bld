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
        exec::{CommonExprExecutor, eval_all_expressions, eval_all_expressions_map},
        traits::EvalExpr,
    },
    external::v3::External,
    runner::v3::state::{ActionState, RootState, State},
    step::v3::{ShellCommand, Step},
    strategy::v3::Strategy,
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
        eval_all_expressions(&expr_exec, &self.expr_regex, value)
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
        value.try_into()
    }

    fn resolve_working_dir(&mut self, working_dir: &Option<String>) -> Result<Option<String>> {
        working_dir
            .as_deref()
            .map(|wd| self.eval_all_expr(wd))
            .transpose()
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
        let working_dir = self.resolve_working_dir(working_dir)?;
        let outputs = self
            .platform
            .shell(self.logger.clone(), &working_dir, &cmd)
            .await?;

        self.state.set_outputs(step_id, outputs)?;

        Ok(())
    }

    async fn steps(&mut self) -> Result<()> {
        debug!("starting execution of action steps");
        let action = self.action.clone();
        for step in &action.steps {
            let Some(strategy) = step.strategy() else {
                self.run_step(step).await?;
                continue;
            };
            self.run_step_with_strategy(step, strategy).await?;
        }
        Ok(())
    }

    async fn run_step_with_strategy(&mut self, step: &Step, strategy: &Strategy) -> Result<()> {
        let exec = CommonExprExecutor::new(&self.action, &self.expr_rctx, &self.state);
        let combinations = strategy.combinations(&exec)?;
        let fail_fast = strategy.resolve_fail_fast(&exec)?;

        let mut errors: Vec<String> = Vec::new();
        for combination in combinations {
            self.state.set_matrix(combination);
            if let Err(e) = self.run_step(step).await {
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

    async fn run_step(&mut self, step: &Step) -> Result<()> {
        let result = match self.condition(step.condition()) {
            Ok(true) => {
                self.state.update_node_state(step.id(), State::Running);
                match step {
                    Step::ComplexSh(complex) => self.complex_shell(complex).await,
                    Step::ExternalFile(external) => self.external(external).await,
                    Step::DownloadArtifact(download) => self.download_artifact(download).await,
                    Step::UploadArtifact(upload) => self.upload_artifact(upload).await,
                }
            }
            Ok(false) => {
                debug!("condition failed, skiping step");
                return Ok(());
            }
            Err(e) => Err(e),
        };

        result
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
        let outputs = runner.run().await?;

        // A step with a strategy runs once per combination, each producing a different
        // map, so no single map is representative and none is stored.
        if details.strategy.is_none() {
            self.state.set_outputs(&details.id, outputs)?;
        }

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
        let expr_exec = CommonExprExecutor::new(&self.action, &self.expr_rctx, &self.state);
        eval_all_expressions_map(&expr_exec, &self.expr_regex, vars)
    }

    fn resolve_outputs(&mut self) -> Result<HashMap<String, String>> {
        let expr_exec = CommonExprExecutor::new(&self.action, &self.expr_rctx, &self.state);
        let outputs = self.action.outputs_map();
        eval_all_expressions_map(&expr_exec, &self.expr_regex, &outputs)
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

    async fn execute(mut self) -> Result<HashMap<String, String>> {
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
        let outputs = self.resolve_outputs().inspect_err(|e| {
            self.state.update_state(State::Failed {
                error: e.to_string(),
            })
        })?;
        self.state.update_state(State::Completed);
        Ok(outputs)
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
    use std::collections::HashMap;

    use bld_config::BldConfig;
    use bld_core::{
        artifacts::Artifacts, context::Context, fs::FileSystem, logger::Logger, platform::Platform,
        regex::RegexCache,
    };
    use bld_pkg::PackageManager;
    use bld_utils::sync::IntoArc;
    use regex::Regex;

    use crate::{
        action::v3::Action,
        artifacts::v3::{DownloadArtifact, UploadArtifact},
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext, parser::EXPR_REGEX,
            traits::WritableRuntimeExprContext,
        },
        external::v3::External,
        outputs::v3::Output,
        runner::v3::{
            ActionRunner, ActionState, RootState, State, state::MockRootState, test_utils::TempDir,
        },
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

    #[test]
    pub fn resolve_working_dir_evaluates_expressions() {
        let logger = Logger::mock().into_arc();
        let action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let inputs: HashMap<String, String> =
            [("worktree_dir".to_string(), "/tmp/some-worktree".to_string())]
                .into_iter()
                .collect();
        let rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        };
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

        assert_eq!(runner.resolve_working_dir(&None).unwrap(), None);

        assert_eq!(
            runner
                .resolve_working_dir(&Some("${{ inputs.worktree_dir }}".to_string()))
                .unwrap(),
            Some("/tmp/some-worktree".to_string())
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
    pub async fn download_and_upload_artifact_steps_execute_with_expr_success() {
        // Arrange
        let logger = Logger::mock().into_arc();
        let mut action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut inputs = HashMap::new();
        inputs.insert("region".to_string(), "eu-west".to_string());
        let rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        };

        action
            .steps
            .push(Step::DownloadArtifact(Box::new(DownloadArtifact {
                id: "download".to_string(),
                download: "artifact-name".to_string(),
                to: "${{ inputs.region }}/artifact".to_string(),
                condition: None,
            })));
        action
            .steps
            .push(Step::UploadArtifact(Box::new(UploadArtifact {
                id: "upload".to_string(),
                upload: "${{ inputs.region }}/artifact".to_string(),
                name: "artifact-name".to_string(),
                condition: None,
            })));

        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
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

        let result = runner.execute().await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn complex_shell_steps_respect_condition_and_still_run_remaining_steps() {
        // Arrange
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

        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "skipped".to_string(),
            name: None,
            run: "echo skipped".to_string(),
            condition: Some("${{ false }}".to_string()),
            working_dir: None,
            strategy: None,
        })));
        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "executed".to_string(),
            name: None,
            run: "echo executed".to_string(),
            condition: Some("${{ true }}".to_string()),
            working_dir: None,
            strategy: None,
        })));

        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
        }

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

        // Act
        let result = runner.steps().await;

        // Assert
        assert!(result.is_ok());
        assert!(matches!(
            runner.state.get_node_state("executed"),
            Some(State::Completed)
        ));
    }

    #[tokio::test]
    pub async fn artifact_steps_respect_condition_and_still_run_remaining_steps() {
        // Arrange
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

        action
            .steps
            .push(Step::DownloadArtifact(Box::new(DownloadArtifact {
                id: "skipped".to_string(),
                download: "artifact-name".to_string(),
                to: "some/path".to_string(),
                condition: Some("${{ false }}".to_string()),
            })));
        action
            .steps
            .push(Step::UploadArtifact(Box::new(UploadArtifact {
                id: "executed".to_string(),
                upload: "some/path".to_string(),
                name: "artifact-name".to_string(),
                condition: Some("${{ true }}".to_string()),
            })));

        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
        }

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

        // Act
        let result = runner.steps().await;

        // Assert
        assert!(result.is_ok(), "error: {:?}", result.err());
        assert!(matches!(
            runner.state.get_node_state("skipped"),
            Some(State::Default)
        ));
        assert!(matches!(
            runner.state.get_node_state("executed"),
            Some(State::Completed)
        ));
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

    #[tokio::test]
    pub async fn action_with_no_outputs_key_gives_empty_map_success() {
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

        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            run: "echo hello".to_string(),
            condition: None,
            working_dir: None,
            strategy: None,
        })));

        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
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

        let result = runner.execute().await;
        assert!(result.is_ok(), "error: {:?}", result.err());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    pub fn resolve_outputs_reads_input_and_step_output_success() {
        let logger = Logger::mock().into_arc();
        let mut action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let mut inputs = HashMap::new();
        inputs.insert("tag".to_string(), "my-image:latest".to_string());
        let rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        };
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        action.steps.push(Step::ComplexSh(Box::new(ShellCommand {
            id: "build".to_string(),
            name: None,
            run: "echo hello".to_string(),
            condition: None,
            working_dir: None,
            strategy: None,
        })));
        action.outputs.insert(
            "image".to_string(),
            Output::Simple("${{ inputs.tag }}".to_string()),
        );
        action.outputs.insert(
            "digest".to_string(),
            Output::Simple("${{ steps.build.outputs.digest }}".to_string()),
        );

        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
        }
        let mut outputs = HashMap::new();
        outputs.insert("digest".to_string(), "sha256:abc".to_string());
        state.set_outputs("build", outputs).unwrap();

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

        let outputs = runner.resolve_outputs().unwrap();
        assert_eq!(outputs.get("image"), Some(&"my-image:latest".to_string()));
        assert_eq!(outputs.get("digest"), Some(&"sha256:abc".to_string()));
    }

    #[test]
    pub fn resolve_outputs_of_a_described_output_gives_only_its_value_success() {
        let logger = Logger::mock().into_arc();
        let mut action = Action::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let mut inputs = HashMap::new();
        inputs.insert("tag".to_string(), "my-image:latest".to_string());
        let rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        };
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        action.outputs.insert(
            "described".to_string(),
            Output::Complex {
                description: Some("The tag of the image that was pushed".to_string()),
                value: "${{ inputs.tag }}".to_string(),
            },
        );
        action.outputs.insert(
            "plain".to_string(),
            Output::Simple("${{ inputs.tag }}".to_string()),
        );

        let mut runner = ActionRunner {
            logger,
            action,
            platform,
            artifacts,
            expr_regex: regex,
            expr_rctx: rctx,
            state: ActionState::default(),
            config,
            fs,
            run_ctx,
            regex_cache,
            package_manager,
        };

        let outputs = runner.resolve_outputs().unwrap();
        assert_eq!(outputs.get("described"), outputs.get("plain"));
        assert_eq!(
            outputs.get("described"),
            Some(&"my-image:latest".to_string())
        );
    }

    #[actix_web::test]
    pub async fn action_calling_action_forwards_outputs_success() {
        let dir = TempDir::new("calling_action_forwards_outputs");
        dir.write(
            "inner.yaml",
            r#"
version: 3
type: action
name: Inner

inputs:
  tag:
    required: true

outputs:
  echoed: ${{ inputs.tag }}

steps:
  - id: noop
    run: echo noop
"#,
        );

        let logger = Logger::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let mut inputs = HashMap::new();
        inputs.insert("tag".to_string(), "my-image:latest".to_string());
        let rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        };
        let config = BldConfig {
            root_dir: dir.root_dir(),
            ..Default::default()
        }
        .into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut action = Action::default();
        let mut with = HashMap::new();
        with.insert("tag".to_string(), "${{ inputs.tag }}".to_string());
        action.steps.push(Step::ExternalFile(Box::new(External {
            id: "call_inner".to_string(),
            uses: "inner.yaml".to_string(),
            with,
            ..Default::default()
        })));
        action.outputs.insert(
            "digest".to_string(),
            Output::Simple("${{ steps.call_inner.outputs.echoed }}".to_string()),
        );

        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
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

        let result = runner.execute().await;
        assert!(result.is_ok(), "error: {:?}", result.err());

        let outputs = result.unwrap();
        assert_eq!(outputs.get("digest"), Some(&"my-image:latest".to_string()));
    }

    #[actix_web::test]
    pub async fn action_calling_action_with_strategy_stores_no_outputs_success() {
        let dir = TempDir::new("calling_action_with_strategy_stores_no_outputs");
        dir.write(
            "inner.yaml",
            r#"
version: 3
type: action
name: Inner

inputs:
  tag:
    required: true

outputs:
  echoed: ${{ inputs.tag }}

steps:
  - id: noop
    run: echo noop
"#,
        );

        let logger = Logger::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex = Regex::new(EXPR_REGEX).unwrap();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let config = BldConfig {
            root_dir: dir.root_dir(),
            ..Default::default()
        }
        .into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut with = HashMap::new();
        with.insert("tag".to_string(), "static-tag".to_string());

        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Array(vec!["linux".to_string(), "windows".to_string()]),
        );

        let mut action = Action::default();
        action.steps.push(Step::ExternalFile(Box::new(External {
            id: "call_inner".to_string(),
            uses: "inner.yaml".to_string(),
            with,
            strategy: Some(Strategy {
                matrix,
                fail_fast: None,
            }),
            ..Default::default()
        })));
        // The calling step ran with a strategy, so no output map was stored for it and
        // reading it back here must fail rather than silently returning one combination's
        // value.
        action.outputs.insert(
            "digest".to_string(),
            Output::Simple("${{ steps.call_inner.outputs.echoed }}".to_string()),
        );

        let mut state = ActionState::default();
        for step in &action.steps {
            state.add_node(step.id());
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

        let result = runner.execute().await;
        assert!(result.is_err());
    }
}
