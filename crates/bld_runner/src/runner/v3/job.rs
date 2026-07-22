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
        context::CommonReadonlyRuntimeExprContext,
        exec::CommonExprExecutor,
        traits::{EvalExpr, ExprValue},
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

        // Evaluate expression in volumes before building platform
        let raw_volumes: Vec<String> = match &job.runs_on {
            RunsOn::Pull { volumes, .. } | RunsOn::Build { volumes, .. } => volumes.clone(),
            _ => Vec::new(),
        };
        let volumes = {
            let exec = CommonExprExecutor::new(
                options.pipeline.as_ref(),
                options.expr_rctx.as_ref(),
                &options.state,
            );
            let mut resolved = Vec::with_capacity(raw_volumes.len());
            for volume in &raw_volumes {
                let mut value = volume.to_string();
                for entry in options.expr_regex.find_iter(volume) {
                    let entry = entry.as_str();
                    let evaluated = exec.eval(entry)?.to_string();
                    value = value.replace(entry, &evaluated);
                }
                resolved.push(value);
            }
            resolved
        };

        let platform = build_platform(
            job,
            options.pipeline.clone(),
            options.config.clone(),
            options.logger.clone(),
            options.run_ctx.clone(),
            options.expr_rctx.clone(),
            volumes,
        )
        .await?;

        Ok(JobRunner { options, platform })
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

        self.info(job).await?;

        if !self.condition(job.condition.as_deref())? {
            debug!("condition failed, skiping step");
            return Ok(self);
        }

        self.options.state.update_state(State::Running);

        debug!("starting execution of pipeline steps");
        let result = self.run_job_steps(job).await;

        match &result {
            Ok(()) => self.options.state.update_state(State::Completed),
            Err(e) => self.options.state.update_state(State::Failed {
                error: e.to_string(),
            }),
        }
        result?;

        self.dispose_platform(job).await?;
        Ok(self)
    }

    async fn run_job_steps(&mut self, job: &Job) -> Result<()> {
        let Some(strategy) = job.strategy.as_ref() else {
            for step in job.steps.iter() {
                self.run_step(step, None).await?;
            }
            return Ok(());
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

    async fn info(&self, job: &Job) -> Result<()> {
        debug!("printing job informantion");
        self.options
            .logger
            .write_line(format!("{:<15}: {}", "Runs on", &job.runs_on))
            .await
    }

    async fn step(&mut self, step: &Step) -> Result<()> {
        self.options
            .state
            .update_node_state(step.id(), State::Running);
        let result = match step {
            Step::ComplexSh(complex) => self.complex_shell(complex).await,
            Step::ExternalFile(external) => self.external(external).await,
            Step::DownloadArtifact(download) => self.download_artifact(download).await,
            Step::UploadArtifact(upload) => self.upload_artifact(upload).await,
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
        let condition = complex.condition.as_deref();

        if !self.condition(condition)? {
            debug!("condition failed, skiping step");
            return Ok(());
        }

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
        runner.run().await?;

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
        let mut result = HashMap::new();
        for (name, value) in vars {
            let value = self.eval_all_expr(value)?;
            result.insert(name.to_string(), value);
        }
        Ok(result)
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

        let outputs = self
            .platform
            .shell(self.options.logger.clone(), working_dir, &command)
            .await?;

        self.options.state.set_outputs(step_id, outputs)?;

        Ok(())
    }

    fn condition(&self, condition: Option<&str>) -> Result<bool> {
        let Some(condition) = condition else {
            return Ok(true);
        };

        debug!("evaluating condition {condition} for step");

        let matches = self.options.expr_regex.find_iter(condition);

        if matches.count() > 1 {
            bail!("more than one condition found for step");
        };

        let expr_exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &self.options.state,
        );
        let value = expr_exec.eval(condition)?;
        Ok(matches!(value, ExprValue::Boolean(true)))
    }

    fn eval_all_expr(&mut self, value: &str) -> Result<String> {
        let expr_exec = CommonExprExecutor::new(
            self.options.pipeline.as_ref(),
            self.options.expr_rctx.as_ref(),
            &self.options.state,
        );

        let mut result = value.to_string();
        for entry in self.options.expr_regex.find_iter(value) {
            let entry = entry.as_str();
            let expr_value = expr_exec.eval(entry)?.to_string();
            result = result.replace(entry, &expr_value);
        }

        Ok(result)
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
    job: &Job,
    pipeline: Arc<Pipeline>,
    config: Arc<BldConfig>,
    logger: Arc<Logger>,
    run_ctx: Arc<Context>,
    expr_rctx: Arc<CommonReadonlyRuntimeExprContext>,
    volumes: Vec<String>,
) -> Result<Arc<Platform>> {
    let options = match &job.runs_on {
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
        .pipeline_env(&pipeline.env)
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

    use std::collections::HashMap;

    use crate::{
        expr::v3::{context::CommonReadonlyRuntimeExprContext, parser::EXPR_REGEX},
        job::v3::Job,
        pipeline::v3::Pipeline,
        runner::v3::{MockRootState, RootState, State, state::JobState},
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
        let job = JobRunner { options, platform };

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
        let runner = JobRunner { options, platform };

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
        let runner = JobRunner { options, platform };

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
        let runner = JobRunner { options, platform };

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
        let runner = JobRunner { options, platform };

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
        let runner = JobRunner { options, platform };

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
        let runner = JobRunner { options, platform };

        let result = runner.run().await;
        assert!(result.is_err());
    }
}
