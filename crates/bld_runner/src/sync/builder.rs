use crate::platform::{Container, Machine, TargetPlatform};
use crate::pipeline::Pipeline;
use crate::pipeline::runs_on::RunsOn;
use crate::sync::runner::Runner;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::context::ContextSender;
use bld_core::execution::Execution;
use bld_core::logger::LoggerSender;
use bld_core::proxies::PipelineFileSystemProxy;
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
    cfg: Option<Arc<BldConfig>>,
    execution: Arc<Execution>,
    logger: Arc<LoggerSender>,
    prx: Arc<PipelineFileSystemProxy>,
    pip: Option<String>,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Option<Arc<HashMap<String, String>>>,
    vars: Option<Arc<HashMap<String, String>>>,
    context: Arc<ContextSender>,
    is_child: bool,
}

impl Default for RunnerBuilder {
    fn default() -> Self {
        Self {
            run_id: Uuid::new_v4().to_string(),
            run_start_time: Local::now().format("%F %X").to_string(),
            cfg: None,
            execution: Execution::default().into_arc(),
            logger: LoggerSender::default().into_arc(),
            prx: PipelineFileSystemProxy::default().into_arc(),
            pip: None,
            ipc: None.into_arc(),
            env: None,
            vars: None,
            context: ContextSender::default().into_arc(),
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

    pub fn config(mut self, cfg: Arc<BldConfig>) -> Self {
        self.cfg = Some(cfg);
        self
    }

    pub fn execution(mut self, ex: Arc<Execution>) -> Self {
        self.execution = ex;
        self
    }

    pub fn logger(mut self, logger: Arc<LoggerSender>) -> Self {
        self.logger = logger;
        self
    }

    pub fn pipeline(mut self, name: &str) -> Self {
        self.pip = Some(name.to_string());
        self
    }

    pub fn proxy(mut self, prx: Arc<PipelineFileSystemProxy>) -> Self {
        self.prx = prx;
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

    pub async fn build(self) -> Result<Runner> {
        let cfg = self
            .cfg
            .ok_or_else(|| anyhow!("no bld config instance provided"))?;
        let pip_name = self.pip.ok_or_else(|| anyhow!("no pipeline provided"))?;
        let pipeline = Pipeline::parse(&self.prx.read(&pip_name)?)?;
        let env = self
            .env
            .ok_or_else(|| anyhow!("no environment instance provided"))?;
        let env = pipeline
            .environment
            .iter()
            .map(|e| {
                (
                    e.name.to_string(),
                    env.get(&e.name).unwrap_or(&e.default_value).to_string(),
                )
            })
            .collect::<HashMap<String, String>>()
            .into_arc();
        let vars = self
            .vars
            .ok_or_else(|| anyhow!("no variables instance provided"))?;
        let vars = pipeline
            .variables
            .iter()
            .map(|v| {
                (
                    v.name.to_string(),
                    vars.get(&v.name).unwrap_or(&v.default_value).to_string(),
                )
            })
            .collect::<HashMap<String, String>>()
            .into_arc();
        let platform = match &pipeline.runs_on {
            RunsOn::Machine => {
                let machine = Machine::new(&self.run_id, env.clone(), self.logger.clone())?;
                TargetPlatform::Machine(Box::new(machine))
            }
            RunsOn::Docker(img) => {
                let container = Container::new(
                    img,
                    cfg.clone(),
                    env.clone(),
                    self.logger.clone(),
                    self.context.clone(),
                )
                .await?;
                TargetPlatform::Container(Box::new(container))
            }
        };
        Ok(Runner {
            run_id: self.run_id,
            run_start_time: self.run_start_time,
            cfg,
            execution: self.execution,
            logger: self.logger,
            prx: self.prx,
            pip: pipeline,
            ipc: self.ipc,
            env,
            vars,
            context: self.context,
            platform,
            is_child: self.is_child,
            has_faulted: false,
        })
    }
}
