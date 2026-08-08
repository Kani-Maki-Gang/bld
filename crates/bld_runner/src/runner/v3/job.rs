use std::{collections::HashMap, fmt::Write, sync::Arc};

use anyhow::{Result, anyhow, bail};
use bld_config::{BldConfig, SshUserAuth};
use bld_core::{
    artifacts::Artifacts,
    context::Context,
    fs::FileSystem,
    logger::Logger,
    platform::{
        Image, Platform, SshAuthOptions, SshConnectOptions,
        builder::{PlatformBuilder, PlatformOptions},
    },
    regex::RegexCache,
};
use bld_models::dtos::ExecClientMessage;
use bld_pkg::PackageManager;
use bld_sock::ExecClient;
use bld_utils::sync::IntoArc;
use regex::Regex;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::{
    RunnerBuilder,
    artifacts::v3::{DownloadArtifact, UploadArtifact},
    expr::v3::{
        context::{CommonReadonlyRuntimeExprContext, START_OF_RUN_WCTX},
        exec::{CommonExprExecutor, eval_all_expressions, eval_all_expressions_map},
        traits::EvalExpr,
    },
    external::v3::External,
    job::v3::Job,
    pipeline::v3::Pipeline,
    registry::v3::Registry,
    runner::v3::state::{JobState, RootState, State},
    runs_on::v3::RunsOn,
    step::v3::{ShellCommand, Step},
};

pub struct JobRunnerOptions<S: RootState> {
    pub job_name: String,
    pub logger: Arc<Logger>,
    pub config: Arc<BldConfig>,
    pub fs: Arc<FileSystem>,
    pub run_ctx: Arc<Context>,
    pub pipeline: Arc<Pipeline>,
    pub regex_cache: Arc<RegexCache>,
    pub expr_regex: Arc<Regex>,
    pub expr_rctx: Arc<CommonReadonlyRuntimeExprContext>,
    pub package_manager: Arc<PackageManager>,
    pub artifacts: Arc<Artifacts>,
    pub is_child: bool,
    pub state: S,
}

pub struct JobRunner<S: RootState> {
    pub options: JobRunnerOptions<S>,
    pub platform: Arc<Platform>,
    pub runs_on: RunsOn,
    pub outputs: HashMap<String, String>,
}

impl<S: RootState> JobRunner<S> {
    pub async fn new(mut options: JobRunnerOptions<S>) -> Result<Self> {
        let cloned_pipeline = options.pipeline.clone();
        let (_, job) = cloned_pipeline
            .jobs
            .iter()
            .find(|(name, _)| **name == options.job_name)
            .ok_or_else(|| anyhow!("unable to find job with name {}", options.job_name))
            .inspect_err(|e| {
                options.state.update_state(State::Failed {
                    error: e.to_string(),
                })
            })?;

        // Every runs_on field is consumed when building the platform, before any step has
        // run, so its expressions are limited to the start of run context.
        let runs_on = Self::resolve_runs_on(job, &options)?;

        let platform = build_platform(
            &runs_on,
            options.config.clone(),
            options.logger.clone(),
            options.run_ctx.clone(),
            options.expr_rctx.clone(),
        )
        .await?;

        Ok(JobRunner {
            options,
            platform,
            runs_on,
            outputs: HashMap::new(),
        })
    }

    fn resolve_runs_on(job: &Job, options: &JobRunnerOptions<S>) -> Result<RunsOn> {
        let exec = CommonExprExecutor::new(
            options.pipeline.as_ref(),
            options.expr_rctx.as_ref(),
            &START_OF_RUN_WCTX,
        );

        job.runs_on
            .resolve(|value| eval_all_expressions(&exec, &options.expr_regex, value))
    }

    pub async fn run(mut self) -> Result<Self> {
        let pipeline = self.options.pipeline.clone();
        let (_, job) = pipeline
            .jobs
            .iter()
            .find(|(name, _)| **name == self.options.job_name)
            .ok_or_else(|| anyhow!("unable to find job with name {}", self.options.job_name))
            .inspect_err(|e| {
                self.options.state.update_state(State::Failed {
                    error: e.to_string(),
                })
            })?;

        self.info().await?;

        if !self.job_condition(job.condition.as_deref())? {
            debug!("condition failed, skiping step");
            return Ok(self);
        }

        self.options.state.update_state(State::Running);

        debug!("starting execution of pipeline steps");
        self.run_job_steps(job).await.inspect_err(|e| {
            self.options.state.update_state(State::Failed {
                error: e.to_string(),
            })
        })?;

        let outputs = self.resolve_outputs(job).inspect_err(|e| {
            self.options.state.update_state(State::Failed {
                error: e.to_string(),
            })
        })?;
        self.outputs = outputs;

        self.options.state.update_state(State::Completed);

        self.dispose_platform(job).await?;
        Ok(self)
    }

    fn resolve_outputs(&mut self, job: &Job) -> Result<HashMap<String, String>> {
        let expr_exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &self.options.state,
        );
        let outputs = job.outputs_map();
        eval_all_expressions_map(&expr_exec, &self.options.expr_regex, &outputs)
    }

    async fn run_job_steps(&mut self, job: &Job) -> Result<()> {
        let Some(strategy) = job.strategy.as_ref() else {
            for step in job.steps.iter() {
                self.run_step(step, None).await?;
            }
            return Ok(());
        };

        // The job's strategy is resolved before any of its steps have run, so the same
        // start of run limits as runs_on apply to it.
        let exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &START_OF_RUN_WCTX,
        );
        let combinations = strategy.combinations(&exec)?;
        let fail_fast = strategy.resolve_fail_fast(&exec)?;

        let mut errors: Vec<String> = Vec::new();
        for combination in combinations {
            for step in job.steps.iter() {
                if let Err(e) = self.run_step(step, Some(&combination)).await {
                    if fail_fast {
                        return Err(e);
                    }
                    errors.push(e.to_string());
                    break;
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(errors.join("\n")))
        }
    }

    async fn run_step(
        &mut self,
        step: &Step,
        job_matrix: Option<&HashMap<String, String>>,
    ) -> Result<()> {
        let Some(strategy) = step.strategy() else {
            if let Some(job_matrix) = job_matrix {
                self.options.state.set_matrix(job_matrix.clone());
            }
            return self.step(step).await;
        };

        let exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &self.options.state,
        );
        let combinations = strategy.combinations(&exec)?;
        let fail_fast = strategy.resolve_fail_fast(&exec)?;

        let mut errors: Vec<String> = Vec::new();
        for combination in combinations {
            let mut merged = job_matrix.cloned().unwrap_or_default();
            merged.extend(combination);
            self.options.state.set_matrix(merged);
            if let Err(e) = self.step(step).await {
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

    async fn info(&self) -> Result<()> {
        debug!("printing job informantion");
        self.options
            .logger
            .write_line(format!("{:<15}: {}", "Runs on", self.runs_on))
            .await
    }

    async fn step(&mut self, step: &Step) -> Result<()> {
        let condition = self.condition(step.condition());
        let result = match condition {
            Ok(false) => {
                debug!("condition failed, skiping step");
                return Ok(());
            }
            Err(e) => Err(e),
            Ok(true) => {
                self.options
                    .state
                    .update_node_state(step.id(), State::Running);
                match step {
                    Step::ComplexSh(complex) => self.complex_shell(complex).await,
                    Step::ExternalFile(external) => self.external(external).await,
                    Step::DownloadArtifact(download) => self.download_artifact(download).await,
                    Step::UploadArtifact(upload) => self.upload_artifact(upload).await,
                }
            }
        };
        result
            .inspect(|_| {
                self.options
                    .state
                    .update_node_state(step.id(), State::Completed)
            })
            .inspect_err(|e| {
                self.options.state.update_node_state(
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
            self.options.logger.write_line(message).await?;
        }
        self.shell(&complex.id, &complex.working_dir, &complex.run)
            .await?;
        Ok(())
    }

    async fn external(&mut self, external: &External) -> Result<()> {
        if let Some(name) = external.name.as_ref() {
            let mut message = String::new();
            writeln!(message, "{:<15}: {name}", "Step")?;
            self.options.logger.write_line(message).await?;
        }

        debug!("calling external pipeline or action {}", external.uses);

        match external.server.as_ref() {
            Some(server) => self.server_external(server, external).await?,
            None => self.local_external(external).await?,
        };

        Ok(())
    }

    async fn download_artifact(&mut self, download: &DownloadArtifact) -> Result<()> {
        let local_path = self.eval_all_expr(&download.to)?;
        self.options
            .artifacts
            .download(self.platform.clone(), &download.download, &local_path)
            .await
    }

    async fn upload_artifact(&mut self, upload: &UploadArtifact) -> Result<()> {
        let local_path = self.eval_all_expr(&upload.upload)?;
        self.options
            .artifacts
            .upload(self.platform.clone(), &upload.name, &local_path)
            .await
    }

    async fn local_external(&mut self, details: &External) -> Result<()> {
        debug!("building runner for child file");

        let inputs = self.variables_external(&details.with)?;
        let env = self.variables_external(&details.env)?;

        let runner = RunnerBuilder::default()
            .run_id(&self.options.expr_rctx.run_id)
            .run_start_time(&self.options.expr_rctx.run_start_time)
            .config(self.options.config.clone())
            .fs(self.options.fs.clone())
            .file(&details.uses)
            .logger(self.options.logger.clone())
            .env(env.into_arc())
            .inputs(inputs.into_arc())
            .context(self.options.run_ctx.clone())
            .platform(self.platform.clone())
            .regex_cache(self.options.regex_cache.clone())
            .package_manager(self.options.package_manager.clone())
            .artifacts(self.options.artifacts.clone())
            .is_child(true)
            .build()
            .await?;

        debug!("starting child file runner");
        let outputs = runner.run().await?;

        // A step with a strategy runs once per combination, each producing a different
        // map, so no single map is representative and none is stored.
        if details.strategy.is_none() {
            self.options.state.set_outputs(&details.id, outputs)?;
        }

        Ok(())
    }

    async fn server_external(&mut self, server: &str, details: &External) -> Result<()> {
        let inputs = self.variables_external(&details.with)?;
        let env = self.variables_external(&details.env)?;

        debug!("establishing web socket connection with server {}", server);

        let client = ExecClient::connect(
            self.options.config.clone(),
            server.to_owned(),
            self.options.logger.clone(),
            self.options.run_ctx.clone(),
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
        let expr_exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &self.options.state,
        );
        eval_all_expressions_map(&expr_exec, &self.options.expr_regex, vars)
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

        let command = self.eval_all_expr(command)?;
        let working_dir = self.resolve_working_dir(working_dir)?;

        debug!("sending command to platform");
        let outputs = self
            .platform
            .shell(self.options.logger.clone(), &working_dir, &command)
            .await?;

        self.options.state.set_outputs(step_id, outputs)?;

        Ok(())
    }

    /// A job's condition is evaluated before any of its steps have run, so it can only use
    /// the start of run expressions.
    fn job_condition(&self, condition: Option<&str>) -> Result<bool> {
        let expr_exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &START_OF_RUN_WCTX,
        );
        self.eval_condition(&expr_exec, condition)
    }

    fn condition(&self, condition: Option<&str>) -> Result<bool> {
        let expr_exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &self.options.state,
        );
        self.eval_condition(&expr_exec, condition)
    }

    fn eval_condition<'a, E: EvalExpr<'a>>(
        &self,
        expr_exec: &E,
        condition: Option<&'a str>,
    ) -> Result<bool> {
        let Some(condition) = condition else {
            return Ok(true);
        };

        debug!("evaluating condition {condition} for step");

        let matches = self.options.expr_regex.find_iter(condition);

        if matches.count() > 1 {
            bail!("more than one condition found for step");
        };

        let value = expr_exec.eval(condition)?;
        value.try_into()
    }

    fn eval_all_expr(&mut self, value: &str) -> Result<String> {
        let expr_exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &self.options.state,
        );

        eval_all_expressions(&expr_exec, &self.options.expr_regex, value)
    }

    async fn dispose_platform(&self, job: &Job) -> Result<()> {
        if job.dispose {
            debug!("executing dispose operations for platform");
            self.platform.dispose(self.options.is_child).await?;
        } else {
            debug!("keeping platform alive");
            self.platform.keep_alive().await?;
        }
        self.options
            .run_ctx
            .remove_platform(self.platform.id())
            .await
    }
}

pub struct RunningJob {
    pub name: String,
    pub handle: JoinHandle<Result<JobRunner<JobState>>>,
    pub logger: Arc<Logger>,
}

impl RunningJob {
    pub fn new(
        name: &str,
        handle: JoinHandle<Result<JobRunner<JobState>>>,
        logger: Arc<Logger>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            handle,
            logger,
        }
    }
}

pub async fn build_platform(
    runs_on: &RunsOn,
    config: Arc<BldConfig>,
    logger: Arc<Logger>,
    run_ctx: Arc<Context>,
    expr_rctx: Arc<CommonReadonlyRuntimeExprContext>,
) -> Result<Arc<Platform>> {
    let volumes = runs_on.volumes().to_vec();

    let options = match runs_on {
        RunsOn::ContainerOrMachine(image) if image == "machine" => PlatformOptions::Machine,

        RunsOn::ContainerOrMachine(image) => PlatformOptions::Container {
            image: Image::Use(image),
            docker_url: None,
            volumes,
        },

        RunsOn::Pull {
            image,
            pull,
            docker_url,
            registry,
            volumes: _,
        } => {
            let image = if pull.unwrap_or_default() {
                match registry.as_ref() {
                    Some(Registry::FromConfig(value)) => Image::pull(image, config.registry(value)),
                    Some(Registry::Full(config)) => Image::pull(image, Some(config)),
                    None => Image::pull(image, None),
                }
            } else {
                Image::Use(image)
            };
            PlatformOptions::Container {
                docker_url: docker_url.as_deref(),
                image,
                volumes,
            }
        }

        RunsOn::Build {
            name,
            tag,
            dockerfile,
            docker_url,
            volumes: _,
        } => PlatformOptions::Container {
            image: Image::build(name, dockerfile, tag),
            docker_url: docker_url.as_deref(),
            volumes,
        },

        RunsOn::SshFromGlobalConfig { ssh_config } => {
            let config = config.ssh(ssh_config)?;
            let port = config.port.parse::<u16>()?;
            let auth = match &config.userauth {
                SshUserAuth::Agent => SshAuthOptions::Agent,
                SshUserAuth::Password { password } => SshAuthOptions::Password { password },
                SshUserAuth::Keys {
                    public_key,
                    private_key,
                } => SshAuthOptions::Keys {
                    public_key: public_key.as_deref(),
                    private_key,
                },
            };
            PlatformOptions::Ssh(SshConnectOptions::new(
                &config.host,
                port,
                &config.user,
                auth,
            ))
        }

        RunsOn::Ssh(config) => {
            let port = config.port.parse::<u16>()?;
            let auth = match &config.userauth {
                SshUserAuth::Agent => SshAuthOptions::Agent,
                SshUserAuth::Password { password } => SshAuthOptions::Password { password },
                SshUserAuth::Keys {
                    public_key,
                    private_key,
                } => SshAuthOptions::Keys {
                    public_key: public_key.as_deref(),
                    private_key,
                },
            };
            PlatformOptions::Ssh(SshConnectOptions::new(
                &config.host,
                port,
                &config.user,
                auth,
            ))
        }
    };

    let conn = run_ctx.get_conn();
    let platform = PlatformBuilder::default()
        .run_id(&expr_rctx.run_id)
        .config(config.clone())
        .options(options)
        .pipeline_env(expr_rctx.env.as_ref())
        .env(expr_rctx.env.clone())
        .logger(logger.clone())
        .conn(conn)
        .build()
        .await?;

    run_ctx.add_platform(platform.clone()).await?;
    Ok(platform)
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

    use anyhow::Result;
    use bld_config::{SshConfig, SshUserAuth};
    use std::collections::HashMap;

    use crate::{
        artifacts::v3::DownloadArtifact,
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext,
            parser::EXPR_REGEX,
            traits::{ExprText, ExprValue, OutputScope, WritableRuntimeExprContext},
        },
        external::v3::External,
        job::v3::Job,
        outputs::v3::Output,
        pipeline::v3::Pipeline,
        runner::v3::{MockRootState, RootState, State, state::JobState, test_utils::TempDir},
        runs_on::v3::RunsOn,
        step::v3::{ShellCommand, Step},
        strategy::v3::{FailFastValue, MatrixValue, Strategy},
    };

    use super::{JobRunner, JobRunnerOptions};

    #[test]
    pub fn condition_eval_success() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let state = JobState::default();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let pipeline = Pipeline::default().into_arc();

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline,
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let job = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        assert!(matches!(job.condition(None), Ok(true)));

        assert!(matches!(job.condition(Some("${{ true }}")), Ok(true)));

        assert!(matches!(
            job.condition(Some("${{ \"John\" == \"James\" }}")),
            Ok(false)
        ));

        assert!(job.condition(Some("${{ true == \"James\" }}")).is_err());

        assert!(
            job.condition(Some("hello world ${{ true == \"James\" }}"))
                .is_err()
        );
    }

    #[test]
    pub fn condition_eval_accepts_text_true_and_false() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let state = JobState::default();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let pipeline = Pipeline::default().into_arc();

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline,
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let job = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        // A text value of "true" starts the step, mirroring an input whose value is "true".
        assert!(matches!(job.condition(Some("${{ \"true\" }}")), Ok(true)));

        // A text value of "false" skips the step, mirroring an input whose value is "false".
        assert!(matches!(job.condition(Some("${{ \"false\" }}")), Ok(false)));

        // A number is not a valid condition value and must produce an error naming the type.
        let number_err = job.condition(Some("${{ 1 }}")).unwrap_err();
        assert!(number_err.to_string().contains("number"));

        // An array is not a valid condition value and must produce an error.
        assert!(job.condition(Some("${{ [1, 2] }}")).is_err());
    }

    #[test]
    pub fn resolve_working_dir_evaluates_expressions() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let inputs: HashMap<String, String> =
            [("worktree_dir".to_string(), "/tmp/some-worktree".to_string())]
                .into_iter()
                .collect();
        let expr_rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        }
        .into_arc();
        let state = JobState::default();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let pipeline = Pipeline::default().into_arc();

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline,
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let mut job = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        assert_eq!(job.resolve_working_dir(&None).unwrap(), None);

        assert_eq!(
            job.resolve_working_dir(&Some("${{ inputs.worktree_dir }}".to_string()))
                .unwrap(),
            Some("/tmp/some-worktree".to_string())
        );
    }

    fn resolve_runs_on(runs_on: RunsOn, inputs: Vec<(&str, &str)>) -> Result<RunsOn> {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();

        let inputs: HashMap<String, String> = inputs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let expr_rctx = CommonReadonlyRuntimeExprContext {
            inputs: inputs.into_arc(),
            ..Default::default()
        }
        .into_arc();

        let state = JobState::default();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                runs_on,
                ..Default::default()
            },
        );
        let pipeline = pipeline.into_arc();
        let job = pipeline.jobs.get(&job_name).unwrap().clone();

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline,
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };

        JobRunner::resolve_runs_on(&job, &options)
    }

    #[test]
    pub fn resolve_runs_on_evaluates_container_image_expr_success() {
        let resolved = resolve_runs_on(
            RunsOn::ContainerOrMachine("${{ inputs.image }}".to_string()),
            vec![("image", "ubuntu:22.04")],
        )
        .unwrap();

        assert!(resolved.volumes().is_empty());
        match resolved {
            RunsOn::ContainerOrMachine(image) => assert_eq!(image, "ubuntu:22.04"),
            other => panic!("expected ContainerOrMachine, got {other:?}"),
        }
    }

    #[test]
    pub fn resolve_runs_on_evaluates_pull_fields_and_volumes_expr_success() {
        let resolved = resolve_runs_on(
            RunsOn::Pull {
                image: "${{ inputs.image }}".to_string(),
                registry: None,
                pull: Some(true),
                docker_url: None,
                volumes: vec!["${{ inputs.worktree_dir }}:${{ inputs.worktree_dir }}".to_string()],
            },
            vec![
                ("image", "my-registry/my-image:latest"),
                ("worktree_dir", "/home/user/worktree"),
            ],
        )
        .unwrap();

        assert_eq!(
            resolved.volumes(),
            ["/home/user/worktree:/home/user/worktree".to_string()]
        );
        match resolved {
            RunsOn::Pull { image, .. } => assert_eq!(image, "my-registry/my-image:latest"),
            other => panic!("expected Pull, got {other:?}"),
        }
    }

    #[test]
    pub fn resolve_runs_on_evaluates_build_and_ssh_fields_expr_success() {
        let resolved = resolve_runs_on(
            RunsOn::Build {
                name: "${{ inputs.name }}".to_string(),
                tag: "${{ inputs.tag }}".to_string(),
                dockerfile: "${{ inputs.dir }}/Dockerfile".to_string(),
                docker_url: None,
                volumes: vec![],
            },
            vec![
                ("name", "my-image"),
                ("tag", "1.0.0"),
                ("dir", "/home/user"),
            ],
        )
        .unwrap();

        match resolved {
            RunsOn::Build {
                name,
                tag,
                dockerfile,
                ..
            } => {
                assert_eq!(name, "my-image");
                assert_eq!(tag, "1.0.0");
                assert_eq!(dockerfile, "/home/user/Dockerfile");
            }
            other => panic!("expected Build, got {other:?}"),
        }

        let resolved = resolve_runs_on(
            RunsOn::Ssh(SshConfig {
                host: "${{ inputs.host }}".to_string(),
                port: "2222".to_string(),
                user: "${{ inputs.user }}".to_string(),
                userauth: SshUserAuth::Password {
                    password: "${{ inputs.password }}".to_string(),
                },
            }),
            vec![
                ("host", "localhost"),
                ("user", "some_user"),
                ("password", "some_password"),
            ],
        )
        .unwrap();

        match resolved {
            RunsOn::Ssh(config) => {
                assert_eq!(config.host, "localhost");
                assert_eq!(config.user, "some_user");
                assert!(
                    matches!(config.userauth, SshUserAuth::Password { password } if password == "some_password")
                );
            }
            other => panic!("expected Ssh, got {other:?}"),
        }
    }

    #[test]
    pub fn resolve_runs_on_with_runtime_expr_failure() {
        let data = vec![
            RunsOn::ContainerOrMachine("${{ steps.build.outputs.image }}".to_string()),
            RunsOn::ContainerOrMachine("${{ matrix.image }}".to_string()),
            RunsOn::Pull {
                image: "ubuntu:22.04".to_string(),
                registry: None,
                pull: Some(true),
                docker_url: None,
                volumes: vec!["${{ matrix.dir }}:/work".to_string()],
            },
        ];

        for runs_on in data {
            let result = resolve_runs_on(runs_on, vec![]);
            assert!(result.is_err(), "expected an error for runtime expression");
        }
    }

    #[tokio::test]
    pub async fn run_state_management_success() {
        // Arrange
        let data: Vec<String> = (0..10).map(|x| format!("step-{x}")).collect();
        let job_name = "test".to_string();
        let logger = Logger::mock().into_arc();
        let config = BldConfig::default().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let mut pipeline = Pipeline::default();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let mut state = MockRootState::new();

        let mut steps = vec![];

        state.expect_get_exec_id().returning(|| Some("test"));

        state
            .expect_update_state()
            .withf(|state| matches!(state, State::Running))
            .return_once(|_| ());

        state.expect_set_matrix().returning(|_| ());

        for node_id in &data {
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

            steps.push(Step::ComplexSh(Box::new(ShellCommand {
                id: node_id.to_string(),
                run: "echo hello".to_string(),
                ..Default::default()
            })));
        }

        state
            .expect_update_state()
            .withf(|state| matches!(state, State::Completed))
            .return_once(|_| ());

        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                steps,
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            run_ctx,
            pipeline: pipeline.into_arc(),
            logger,
            config,
            fs,
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        // Act
        runner.run().await.unwrap();
    }

    #[tokio::test]
    pub async fn job_level_matrix_expansion_runs_all_combinations_success() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let mut state = JobState::new(&job_name);
        state.add_node("build");

        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Array(vec!["linux".to_string(), "windows".to_string()]),
        );
        matrix.insert(
            "version".to_string(),
            MatrixValue::Array(vec!["v2".to_string(), "v3".to_string()]),
        );

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                strategy: Some(Strategy {
                    matrix,
                    fail_fast: None,
                }),
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo ${{ matrix.os }} ${{ matrix.version }}".to_string(),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        let result = runner.run().await;
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[tokio::test]
    pub async fn step_level_matrix_expansion_runs_all_combinations_success() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let mut state = JobState::new(&job_name);
        state.add_node("build");

        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Array(vec!["linux".to_string(), "windows".to_string()]),
        );

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo ${{ matrix.os }}".to_string(),
                    strategy: Some(Strategy {
                        matrix,
                        fail_fast: None,
                    }),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        let result = runner.run().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn combined_job_and_step_matrix_full_cartesian_success() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let mut state = JobState::new(&job_name);
        state.add_node("build");

        let mut job_matrix = HashMap::new();
        job_matrix.insert(
            "os".to_string(),
            MatrixValue::Array(vec!["linux".to_string(), "windows".to_string()]),
        );

        let mut step_matrix = HashMap::new();
        step_matrix.insert(
            "version".to_string(),
            MatrixValue::Array(vec!["v2".to_string(), "v3".to_string()]),
        );

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                strategy: Some(Strategy {
                    matrix: job_matrix,
                    fail_fast: None,
                }),
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo ${{ matrix.os }} ${{ matrix.version }}".to_string(),
                    strategy: Some(Strategy {
                        matrix: step_matrix,
                        fail_fast: None,
                    }),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        let result = runner.run().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn job_matrix_fail_fast_true_stops_at_first_failure() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
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

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                strategy: Some(Strategy {
                    matrix,
                    fail_fast: None,
                }),
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo hello".to_string(),
                    condition: Some("${{ 1 }} ${{ 2 }}".to_string()),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        let result = runner.run().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    pub async fn job_matrix_fail_fast_false_runs_all_and_aggregates() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
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

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                strategy: Some(Strategy {
                    matrix,
                    fail_fast: Some(FailFastValue::Bool(false)),
                }),
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo hello".to_string(),
                    condition: Some("${{ 1 }} ${{ 2 }}".to_string()),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        let result = runner.run().await;
        assert!(result.is_err());
    }

    const ACTION_WITH_OUTPUT: &str = r#"
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
"#;

    fn job_runner_calling_action(dir: &TempDir, step: External) -> JobRunner<JobState> {
        let job_name = "main".to_string();
        let config = BldConfig {
            root_dir: dir.root_dir(),
            ..Default::default()
        }
        .into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut state = JobState::new(&job_name);
        state.add_node(&step.id);

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                steps: vec![Step::ExternalFile(Box::new(step))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        }
    }

    #[tokio::test]
    pub async fn download_artifact_step_with_false_condition_is_skipped() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let mut state = JobState::new(&job_name);
        state.add_node("download");

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                steps: vec![Step::DownloadArtifact(Box::new(DownloadArtifact {
                    id: "download".to_string(),
                    download: "artifact-name".to_string(),
                    to: "some/path".to_string(),
                    condition: Some("${{ false }}".to_string()),
                }))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        let result = runner.run().await;
        assert!(result.is_ok(), "error: {:?}", result.err());

        let runner = result.unwrap();
        assert!(matches!(
            runner.options.state.get_node_state("download"),
            Some(State::Default)
        ));
    }

    #[tokio::test]
    pub async fn download_artifact_step_with_true_condition_runs() {
        let job_name = "main".to_string();
        let config = BldConfig::default().into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let mut state = JobState::new(&job_name);
        state.add_node("download");

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.clone(),
            Job {
                steps: vec![Step::DownloadArtifact(Box::new(DownloadArtifact {
                    id: "download".to_string(),
                    download: "artifact-name".to_string(),
                    to: "some/path".to_string(),
                    condition: Some("${{ true }}".to_string()),
                }))],
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name,
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        let runner = JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        };

        let result = runner.run().await;
        assert!(result.is_ok(), "error: {:?}", result.err());

        let runner = result.unwrap();
        assert!(matches!(
            runner.options.state.get_node_state("download"),
            Some(State::Completed)
        ));
    }

    #[actix_web::test]
    pub async fn job_step_calling_action_stores_its_outputs_success() {
        let dir = TempDir::new("job_step_calling_action_stores_its_outputs");
        dir.write("inner.yaml", ACTION_WITH_OUTPUT);

        let mut with = HashMap::new();
        with.insert("tag".to_string(), "my-image:latest".to_string());

        let runner = job_runner_calling_action(
            &dir,
            External {
                id: "call_action".to_string(),
                uses: "inner.yaml".to_string(),
                with,
                ..Default::default()
            },
        );

        let result = runner.run().await;
        assert!(result.is_ok(), "error: {:?}", result.err());

        let runner = result.unwrap();
        let output = runner
            .options
            .state
            .get_output(OutputScope::Step, "call_action", "echoed");
        assert_eq!(
            output.unwrap(),
            ExprValue::Text(ExprText::Owned("my-image:latest".to_string()))
        );
    }

    #[actix_web::test]
    pub async fn job_step_with_strategy_calling_action_stores_no_outputs_success() {
        let dir = TempDir::new("job_step_with_strategy_calling_action_stores_no_outputs");
        dir.write("inner.yaml", ACTION_WITH_OUTPUT);

        let mut with = HashMap::new();
        with.insert("tag".to_string(), "${{ matrix.tag }}".to_string());

        let mut matrix = HashMap::new();
        matrix.insert(
            "tag".to_string(),
            MatrixValue::Array(vec!["one".to_string(), "two".to_string()]),
        );

        let runner = job_runner_calling_action(
            &dir,
            External {
                id: "call_action".to_string(),
                uses: "inner.yaml".to_string(),
                with,
                strategy: Some(Strategy {
                    matrix,
                    fail_fast: None,
                }),
                ..Default::default()
            },
        );

        let result = runner.run().await;
        assert!(result.is_ok(), "error: {:?}", result.err());

        // Each combination produces a different map, so none is stored and reading one
        // back gives an error instead of the value of a single combination.
        let runner = result.unwrap();
        assert!(
            runner
                .options
                .state
                .get_output(OutputScope::Step, "call_action", "echoed")
                .is_err()
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn job_runner_with_outputs(
        dir: &TempDir,
        job_name: &str,
        step: External,
        job_outputs: HashMap<String, Output>,
        needs: Option<crate::job::v3::Needs>,
        incoming_job_outputs: HashMap<String, HashMap<String, String>>,
    ) -> JobRunner<JobState> {
        let config = BldConfig {
            root_dir: dir.root_dir(),
            ..Default::default()
        }
        .into_arc();
        let logger = Logger::mock().into_arc();
        let fs = FileSystem::local(config.clone()).into_arc();
        let run_ctx = Context::mock().into_arc();
        let platform = Platform::mock().into_arc();
        let artifacts = Artifacts::mock().into_arc();
        let regex_cache = RegexCache::mock().into_arc();
        let expr_regex = Regex::new(EXPR_REGEX).unwrap().into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default().into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();

        let mut state = JobState::new(job_name);
        state.add_node(&step.id);
        state.set_job_outputs(incoming_job_outputs).unwrap();

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.to_string(),
            Job {
                needs,
                steps: vec![Step::ExternalFile(Box::new(step))],
                outputs: job_outputs,
                ..Default::default()
            },
        );

        let options = JobRunnerOptions {
            job_name: job_name.to_string(),
            logger,
            config,
            fs,
            run_ctx,
            pipeline: pipeline.into_arc(),
            regex_cache,
            expr_regex,
            expr_rctx,
            package_manager,
            artifacts,
            is_child: false,
            state,
        };
        JobRunner {
            options,
            platform,
            runs_on: RunsOn::default(),
            outputs: HashMap::new(),
        }
    }

    /// Two jobs, the second in the needs of the first: the second job's own `outputs` key
    /// reads a value produced by the first job through `jobs.<name>.outputs.<name>`, and the
    /// value it resolves is exactly the one the first job produced.
    #[actix_web::test]
    pub async fn job_reads_output_of_a_job_listed_in_its_needs_success() {
        let dir = TempDir::new("job_reads_output_of_a_job_listed_in_its_needs");
        dir.write("inner.yaml", ACTION_WITH_OUTPUT);

        let mut with = HashMap::new();
        with.insert("tag".to_string(), "my-image:latest".to_string());

        let mut build_outputs = HashMap::new();
        build_outputs.insert(
            "version".to_string(),
            Output::Simple("${{ steps.call_action.outputs.echoed }}".to_string()),
        );

        let build = job_runner_with_outputs(
            &dir,
            "build",
            External {
                id: "call_action".to_string(),
                uses: "inner.yaml".to_string(),
                with,
                ..Default::default()
            },
            build_outputs,
            None,
            HashMap::new(),
        );

        let build = build.run().await.unwrap();
        assert_eq!(
            build.outputs.get("version").map(String::as_str),
            Some("my-image:latest")
        );

        let mut incoming = HashMap::new();
        incoming.insert("build".to_string(), build.outputs.clone());

        let mut publish_outputs = HashMap::new();
        publish_outputs.insert(
            "got".to_string(),
            Output::Simple("${{ jobs.build.outputs.version }}".to_string()),
        );

        let publish = job_runner_with_outputs(
            &dir,
            "publish",
            External {
                id: "call_action".to_string(),
                uses: "inner.yaml".to_string(),
                with: {
                    let mut with = HashMap::new();
                    with.insert("tag".to_string(), "unrelated".to_string());
                    with
                },
                ..Default::default()
            },
            publish_outputs,
            Some(crate::job::v3::Needs::Single("build".to_string())),
            incoming,
        );

        let publish = publish.run().await.unwrap();
        assert_eq!(
            publish.outputs.get("got").map(String::as_str),
            Some("my-image:latest")
        );
    }

    /// A job that never runs because its condition fails still resolves to an empty map of
    /// outputs, and a job that reads one of its values gets a clear error rather than a
    /// stale or default value.
    #[actix_web::test]
    pub async fn job_reading_output_of_a_skipped_job_fails_clearly() {
        let dir = TempDir::new("job_reading_output_of_a_skipped_job");
        dir.write("inner.yaml", ACTION_WITH_OUTPUT);

        let mut incoming = HashMap::new();
        // Mirrors what the pipeline runner stores for a job that was skipped: present in
        // the map, but with no outputs of its own.
        incoming.insert("build".to_string(), HashMap::new());

        let mut publish_outputs = HashMap::new();
        publish_outputs.insert(
            "got".to_string(),
            Output::Simple("${{ jobs.build.outputs.version }}".to_string()),
        );

        let mut with = HashMap::new();
        with.insert("tag".to_string(), "unrelated".to_string());

        let publish = job_runner_with_outputs(
            &dir,
            "publish",
            External {
                id: "call_action".to_string(),
                uses: "inner.yaml".to_string(),
                with,
                ..Default::default()
            },
            publish_outputs,
            Some(crate::job::v3::Needs::Single("build".to_string())),
            incoming,
        );

        let result = publish.run().await;
        let error = result.err().expect("expected an error").to_string();
        assert!(
            error.contains("version") && error.contains("build"),
            "{error}"
        );
    }
}
