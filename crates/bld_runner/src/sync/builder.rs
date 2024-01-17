use super::versioned::VersionedRunner;
use crate::pipeline::traits::Load;
use crate::pipeline::versioned::{VersionedPipeline, Yaml};
use crate::runner::v1;
use crate::runner::v2;
use crate::token_context::v2::PipelineContextBuilder;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::context::ContextSender;
use bld_core::logger::LoggerSender;
use bld_core::messages::WorkerMessages;
use bld_core::platform::{Image, builder::{PlatformBuilder, PlatformOptions}};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::regex::RegexCache;
use bld_core::signals::UnixSignalsReceiver;
use bld_utils::sync::IntoArc;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub struct RunnerBuilder {
    run_id: String,
    run_start_time: String,
    config: Option<Arc<BldConfig>>,
    signals: Option<UnixSignalsReceiver>,
    logger: Arc<LoggerSender>,
    regex_cache: Arc<RegexCache>,
    proxy: Arc<PipelineFileSystemProxy>,
    pipeline: Option<String>,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Option<Arc<HashMap<String, String>>>,
    vars: Option<Arc<HashMap<String, String>>>,
    context: Option<Arc<ContextSender>>,
    is_child: bool,
}

impl Default for RunnerBuilder {
    fn default() -> Self {
        Self {
            run_id: Uuid::new_v4().to_string(),
            run_start_time: Utc::now().naive_utc().format("%F %X").to_string(),
            config: None,
            signals: None,
            logger: LoggerSender::default().into_arc(),
            regex_cache: RegexCache::default().into_arc(),
            proxy: PipelineFileSystemProxy::default().into_arc(),
            pipeline: None,
            ipc: None.into_arc(),
            env: None,
            vars: None,
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

    pub fn signals(mut self, signals: UnixSignalsReceiver) -> Self {
        self.signals = Some(signals);
        self
    }

    pub fn logger(mut self, logger: Arc<LoggerSender>) -> Self {
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

    pub fn proxy(mut self, proxy: Arc<PipelineFileSystemProxy>) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn ipc(mut self, sender: Arc<Option<Sender<WorkerMessages>>>) -> Self {
        self.ipc = sender;
        self
    }

    pub fn environment(mut self, env: Arc<HashMap<String, String>>) -> Self {
        self.env = Some(env);
        self
    }

    pub fn variables(mut self, vars: Arc<HashMap<String, String>>) -> Self {
        self.vars = Some(vars);
        self
    }

    pub fn context(mut self, context: Arc<ContextSender>) -> Self {
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

        let pipeline = Yaml::load(&self.proxy.read(&pipeline_name).await?)?;
        pipeline
            .validate(config.clone(), self.proxy.clone())
            .await?;

        let env = self
            .env
            .ok_or_else(|| anyhow!("no environment instance provided"))?;

        let vars = self
            .vars
            .ok_or_else(|| anyhow!("no variables instance provided"))?;

        let context = self
            .context
            .ok_or_else(|| anyhow!("no context instance provided"))?;

        let runner = match pipeline {
            VersionedPipeline::Version1(pipeline) => {
                let options = match pipeline.runs_on.as_str() {
                    "machine" => PlatformOptions::Machine,
                    image => PlatformOptions::Container {
                        image: Image::Use(image),
                        docker_url: None
                    }
                };

                let platform = PlatformBuilder::default()
                    .run_id(&self.run_id)
                    .options(options)
                    .config(config.clone())
                    .pipeline_environment(&pipeline.environment)
                    .environment(env.clone())
                    .logger(self.logger.clone())
                    .context(context.clone())
                    .build()
                    .await?;

                context.add_platform(platform.clone()).await?;

                VersionedRunner::V1(v1::Runner {
                    run_id: self.run_id,
                    run_start_time: self.run_start_time,
                    config,
                    signals: self.signals,
                    logger: self.logger,
                    proxy: self.proxy,
                    pipeline,
                    ipc: self.ipc,
                    env,
                    vars,
                    context,
                    platform,
                    is_child: self.is_child,
                    has_faulted: false,
                })
            }

            VersionedPipeline::Version2(mut pipeline) => {
                let pipeline_context = PipelineContextBuilder::default()
                    .root_dir(&config.root_dir)
                    .project_dir(&config.project_dir)
                    .add_variables(&pipeline.variables)
                    .add_variables(&vars)
                    .add_environment(&pipeline.environment)
                    .add_environment(&env)
                    .run_id(&self.run_id)
                    .run_start_time(&self.run_start_time)
                    .regex_cache(self.regex_cache.clone())
                    .build()?;

                pipeline.apply_tokens(&pipeline_context).await?;

                VersionedRunner::V2(v2::Runner {
                    run_id: self.run_id,
                    run_start_time: self.run_start_time,
                    config,
                    signals: self.signals,
                    logger: self.logger,
                    regex_cache: self.regex_cache,
                    proxy: self.proxy,
                    pipeline: pipeline.into_arc(),
                    ipc: self.ipc,
                    env,
                    context,
                    platform: None,
                    is_child: self.is_child,
                    has_faulted: false,
                })
            }
        };

        Ok(runner)
    }
}
