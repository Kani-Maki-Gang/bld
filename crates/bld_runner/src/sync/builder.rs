use super::runner::RunnerV1;
use super::versioned::VersionedRunner;
use crate::pipeline::traits::Load;
use crate::pipeline::{VersionedPipeline, Yaml};
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::context::ContextSender;
use bld_core::execution::Execution;
use bld_core::logger::LoggerSender;
use bld_core::platform::{Container, Machine, TargetPlatform};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::signals::UnixSignalsReceiver;
use bld_sock::messages::WorkerMessages;
use bld_utils::sync::IntoArc;
use chrono::offset::Local;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub struct RunnerBuilder {
    run_id: String,
    run_start_time: String,
    config: Option<Arc<BldConfig>>,
    execution: Arc<Execution>,
    signals: Option<UnixSignalsReceiver>,
    logger: Arc<LoggerSender>,
    proxy: Arc<PipelineFileSystemProxy>,
    pipeline: Option<String>,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Option<Arc<HashMap<String, String>>>,
    vars: Option<Arc<HashMap<String, String>>>,
    context: Arc<ContextSender>,
    is_child: bool,
}

impl Default for RunnerBuilder {
    fn default() -> Self {
        let run_id = Uuid::new_v4().to_string();
        let context = ContextSender::local(&run_id).into_arc();
        Self {
            run_id,
            run_start_time: Local::now().format("%F %X").to_string(),
            config: None,
            execution: Execution::default().into_arc(),
            signals: None,
            logger: LoggerSender::default().into_arc(),
            proxy: PipelineFileSystemProxy::default().into_arc(),
            pipeline: None,
            ipc: None.into_arc(),
            env: None,
            vars: None,
            context,
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

    pub fn execution(mut self, ex: Arc<Execution>) -> Self {
        self.execution = ex;
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
        self.context = context;
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
        let pipeline = Yaml::load(&self.proxy.read(&pipeline_name)?)?;
        pipeline.validate(config.clone(), self.proxy.clone())?;

        let env = self
            .env
            .ok_or_else(|| anyhow!("no environment instance provided"))?;

        let vars = self
            .vars
            .ok_or_else(|| anyhow!("no variables instance provided"))?;

        let platform = match pipeline.runs_on() {
            "machine" => {
                let machine = Machine::new(&self.run_id, env.clone(), self.logger.clone())?;
                TargetPlatform::machine(Box::new(machine))
            }
            image => {
                let container = Container::new(
                    image,
                    config.clone(),
                    env.clone(),
                    self.logger.clone(),
                    self.context.clone(),
                )
                .await?;
                TargetPlatform::container(Box::new(container))
            }
        }
        .into_arc();
        self.context.add_platform(platform.clone()).await?;

        let runner = match pipeline {
            VersionedPipeline::Version1(pipeline) => VersionedRunner::Version1(RunnerV1 {
                run_id: self.run_id,
                run_start_time: self.run_start_time,
                config,
                execution: self.execution,
                signals: self.signals,
                logger: self.logger,
                proxy: self.proxy,
                pipeline,
                ipc: self.ipc,
                env,
                vars,
                context: self.context,
                platform,
                is_child: self.is_child,
                has_faulted: false,
            }),
        };

        Ok(runner)
    }
}
