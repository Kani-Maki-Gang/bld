use anyhow::{Result, anyhow, bail};
use bld_config::BldConfig;
use bld_core::{
    context::Context,
    fs::FileSystem,
    logger::Logger,
    platform::{
        Image, Platform,
        builder::{PlatformBuilder, PlatformOptions},
    },
    regex::RegexCache,
    signals::UnixSignalsBackend,
};
use bld_models::dtos::WorkerMessages;
use bld_utils::sync::IntoArc;
use chrono::Utc;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::{
    expr,
    files::{
        v3::RunnerFile,
        versioned::{VersionedFile, Yaml},
    },
    runner::{self, v3::FileRunner, versioned::VersionedRunner},
    token_context,
    traits::Load,
};

use super::v3::build_platform;

pub struct RunnerBuilder<'a> {
    run_id: String,
    run_start_time: String,
    config: Option<Arc<BldConfig>>,
    signals: Option<UnixSignalsBackend>,
    logger: Arc<Logger>,
    regex_cache: Arc<RegexCache>,
    fs: Arc<FileSystem>,
    file: Option<&'a str>,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Option<Arc<HashMap<String, String>>>,
    inputs: Option<Arc<HashMap<String, String>>>,
    context: Option<Arc<Context>>,
    platform: Option<Arc<Platform>>,
    is_child: bool,
}

impl Default for RunnerBuilder<'_> {
    fn default() -> Self {
        Self {
            run_id: Uuid::new_v4().to_string(),
            run_start_time: Utc::now().naive_utc().format("%F %X").to_string(),
            config: None,
            signals: None,
            logger: Logger::default().into_arc(),
            regex_cache: RegexCache::default().into_arc(),
            fs: FileSystem::default().into_arc(),
            file: None,
            ipc: None.into_arc(),
            env: None,
            inputs: None,
            context: None,
            platform: None,
            is_child: false,
        }
    }
}

impl<'a> RunnerBuilder<'a> {
    pub fn run_id(mut self, id: &str) -> Self {
        self.run_id = String::from(id);
        self
    }

    pub fn run_start_time(mut self, time: &str) -> Self {
        self.run_start_time = String::from(time);
        self
    }

    pub fn config(mut self, config: Arc<BldConfig>) -> Self {
        self.config = Some(config);
        self
    }

    pub fn signals(mut self, signals: UnixSignalsBackend) -> Self {
        self.signals = Some(signals);
        self
    }

    pub fn logger(mut self, logger: Arc<Logger>) -> Self {
        self.logger = logger;
        self
    }

    pub fn regex_cache(mut self, regex_cache: Arc<RegexCache>) -> Self {
        self.regex_cache = regex_cache;
        self
    }

    pub fn file(mut self, instance: &'a str) -> Self {
        self.file = Some(instance);
        self
    }

    pub fn fs(mut self, fs: Arc<FileSystem>) -> Self {
        self.fs = fs;
        self
    }

    pub fn ipc(mut self, sender: Arc<Option<Sender<WorkerMessages>>>) -> Self {
        self.ipc = sender;
        self
    }

    pub fn env(mut self, env: Arc<HashMap<String, String>>) -> Self {
        self.env = Some(env);
        self
    }

    pub fn inputs(mut self, inputs: Arc<HashMap<String, String>>) -> Self {
        self.inputs = Some(inputs);
        self
    }

    pub fn context(mut self, context: Arc<Context>) -> Self {
        self.context = Some(context);
        self
    }

    pub fn platform(mut self, platform: Arc<Platform>) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn is_child(mut self, is_child: bool) -> Self {
        self.is_child = is_child;
        self
    }

    pub async fn build(self) -> Result<VersionedRunner> {
        let config = self
            .config
            .ok_or_else(|| anyhow!("no bld config instance provided"))?;

        let pipeline = self.file.ok_or_else(|| anyhow!("no pipeline provided"))?;

        let pipeline = Yaml::load(&self.fs.read(pipeline).await?)?;
        pipeline.validate(config.clone(), self.fs.clone()).await?;

        let env = self
            .env
            .ok_or_else(|| anyhow!("no env instance provided"))?;

        let inputs = self
            .inputs
            .ok_or_else(|| anyhow!("no inputs instance provided"))?;

        let context = self
            .context
            .ok_or_else(|| anyhow!("no context instance provided"))?;

        let runner = match pipeline {
            VersionedFile::Version1(pipeline) => {
                let options = match pipeline.runs_on.as_str() {
                    "machine" => PlatformOptions::Machine,
                    image => PlatformOptions::Container {
                        image: Image::Use(image),
                        docker_url: None,
                    },
                };

                let conn = context.get_conn();
                let platform = PlatformBuilder::default()
                    .run_id(&self.run_id)
                    .options(options)
                    .config(config.clone())
                    .pipeline_env(&pipeline.environment)
                    .env(env.clone())
                    .logger(self.logger.clone())
                    .conn(conn)
                    .build()
                    .await?;

                context.add_platform(platform.clone()).await?;

                VersionedRunner::V1(runner::v1::Runner {
                    run_id: self.run_id,
                    run_start_time: self.run_start_time,
                    config,
                    signals: self.signals,
                    logger: self.logger,
                    fs: self.fs,
                    pipeline,
                    ipc: self.ipc,
                    env,
                    vars: inputs,
                    context,
                    platform,
                    is_child: self.is_child,
                    has_faulted: false,
                })
            }

            VersionedFile::Version2(mut pipeline) => {
                let pipeline_context = token_context::v2::PipelineContextBuilder::default()
                    .root_dir(&config.root_dir)
                    .project_dir(&config.project_dir)
                    .add_variables(&pipeline.variables)
                    .add_variables(&inputs)
                    .add_environment(&pipeline.environment)
                    .add_environment(&env)
                    .run_id(&self.run_id)
                    .run_start_time(&self.run_start_time)
                    .regex_cache(self.regex_cache.clone())
                    .build()?;

                pipeline.apply_tokens(&pipeline_context).await?;

                VersionedRunner::V2(runner::v2::Runner {
                    run_id: self.run_id,
                    run_start_time: self.run_start_time,
                    config,
                    signals: self.signals,
                    logger: self.logger,
                    regex_cache: self.regex_cache,
                    fs: self.fs,
                    pipeline: pipeline.into_arc(),
                    ipc: self.ipc,
                    env,
                    context,
                    platform: None,
                    is_child: self.is_child,
                    has_faulted: false,
                })
            }

            VersionedFile::Version3(RunnerFile::PipelineFileType(pipeline)) => {
                let pipeline = (*pipeline).into_arc();
                let expr_rctx = expr::v3::context::CommonReadonlyRuntimeExprContext::new(
                    config.clone(),
                    inputs,
                    env,
                    self.run_id,
                    self.run_start_time,
                )
                .into_arc();

                let expr_regex = Regex::new(expr::v3::parser::EXPR_REGEX)?.into_arc();

                let platform = build_platform(
                    pipeline.clone(),
                    config.clone(),
                    self.logger.clone(),
                    context.clone(),
                    expr_rctx.clone(),
                )
                .await?;

                VersionedRunner::V3(FileRunner::Pipeline(runner::v3::PipelineRunner {
                    config,
                    expr_regex,
                    expr_rctx,
                    pipeline,
                    platform,
                    run_ctx: context,
                    fs: self.fs.clone(),
                    regex_cache: self.regex_cache.clone(),
                    signals: self.signals,
                    logger: self.logger,
                    ipc: self.ipc,
                    is_child: self.is_child,
                    has_faulted: false,
                }))
            }

            VersionedFile::Version3(RunnerFile::ActionFileType(action)) => {
                if !self.is_child {
                    bail!("cannot run action files");
                }

                let expr_regex = Regex::new(expr::v3::parser::EXPR_REGEX)?;

                let expr_rctx = expr::v3::context::CommonReadonlyRuntimeExprContext::new(
                    config.clone(),
                    inputs,
                    env,
                    self.run_id,
                    self.run_start_time,
                );

                let platform = self
                    .platform
                    .ok_or_else(|| anyhow!("no platform provided"))?;

                VersionedRunner::V3(FileRunner::Action(runner::v3::ActionRunner::new(
                    self.logger,
                    *action,
                    platform,
                    expr_regex,
                    expr_rctx,
                )))
            }
        };

        Ok(runner)
    }
}
