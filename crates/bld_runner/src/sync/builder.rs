use anyhow::{anyhow, bail, Result};
use bld_config::BldConfig;
use bld_core::{
    context::Context,
    fs::FileSystem,
    logger::Logger,
    platform::{
        builder::{PlatformBuilder, PlatformOptions},
        Image,
    },
    regex::RegexCache,
    signals::UnixSignalsBackend,
};
use bld_models::dtos::WorkerMessages;
use bld_utils::sync::IntoArc;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::{
    files::{
        v3::RunnerFile,
        versioned::{VersionedFile, Yaml},
    },
    runner, token_context,
    traits::Load,
};

use super::versioned::VersionedRunner;

pub struct RunnerBuilder {
    run_id: String,
    run_start_time: String,
    config: Option<Arc<BldConfig>>,
    signals: Option<UnixSignalsBackend>,
    logger: Arc<Logger>,
    regex_cache: Arc<RegexCache>,
    fs: Arc<FileSystem>,
    pipeline: Option<String>,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Option<Arc<HashMap<String, String>>>,
    inputs: Option<Arc<HashMap<String, String>>>,
    context: Option<Arc<Context>>,
    is_child: bool,
}

impl Default for RunnerBuilder {
    fn default() -> Self {
        Self {
            run_id: Uuid::new_v4().to_string(),
            run_start_time: Utc::now().naive_utc().format("%F %X").to_string(),
            config: None,
            signals: None,
            logger: Logger::default().into_arc(),
            regex_cache: RegexCache::default().into_arc(),
            fs: FileSystem::default().into_arc(),
            pipeline: None,
            ipc: None.into_arc(),
            env: None,
            inputs: None,
            context: None,
            is_child: false,
        }
    }
}

impl RunnerBuilder {
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

    pub fn pipeline(mut self, name: &str) -> Self {
        self.pipeline = Some(name.to_string());
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

    pub fn is_child(mut self, is_child: bool) -> Self {
        self.is_child = is_child;
        self
    }

    pub async fn build(self) -> Result<VersionedRunner> {
        let config = self
            .config
            .ok_or_else(|| anyhow!("no bld config instance provided"))?;

        let pipeline_name = self
            .pipeline
            .ok_or_else(|| anyhow!("no pipeline provided"))?;

        let pipeline = Yaml::load(&self.fs.read(&pipeline_name).await?)?;
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

            VersionedFile::Version3(RunnerFile::PipelineFileType(mut pipeline)) => {
                let pipeline_context = token_context::v3::PipelineContextBuilder::default()
                    .root_dir(&config.root_dir)
                    .project_dir(&config.project_dir)
                    .add_inputs(&pipeline.inputs)
                    .add_inputs(&inputs)
                    .add_env(&pipeline.env)
                    .add_env(&env)
                    .run_id(&self.run_id)
                    .run_start_time(&self.run_start_time)
                    .regex_cache(self.regex_cache.clone())
                    .build()?;

                pipeline.apply_tokens(&pipeline_context).await?;

                let pipeline = Arc::new(*pipeline);
                VersionedRunner::V3(runner::v3::Runner {
                    run_id: self.run_id,
                    run_start_time: self.run_start_time,
                    config,
                    signals: self.signals,
                    logger: self.logger,
                    regex_cache: self.regex_cache,
                    fs: self.fs,
                    pipeline,
                    ipc: self.ipc,
                    env,
                    context,
                    platform: None,
                    is_child: self.is_child,
                    has_faulted: false,
                })
            }

            VersionedFile::Version3(RunnerFile::ActionFileType) => {
                bail!("cannot run action files");
            }
        };

        Ok(runner)
    }
}
