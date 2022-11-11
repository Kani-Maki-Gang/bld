use crate::{BuildStep, Container, Machine, Pipeline, RunsOn, TargetPlatform};
use actix::{io::SinkWrite, Actor, StreamHandler};
use anyhow::{anyhow, bail, Result};
use bld_config::definitions::{
    ENV_TOKEN, GET, PUSH, RUN_PROPS_ID, RUN_PROPS_START_TIME, VAR_TOKEN,
};
use bld_config::BldConfig;
use bld_core::context::ContextSender;
use bld_core::execution::Execution;
use bld_core::logger::LoggerSender;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_sock::clients::ExecClient;
use bld_sock::messages::{RunInfo, WorkerMessages};
use bld_utils::request::headers;
use bld_utils::sync::IntoArc;
use bld_utils::tls::awc_client;
use chrono::offset::Local;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::debug;
use uuid::Uuid;

type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

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

pub struct Runner {
    run_id: String,
    run_start_time: String,
    cfg: Arc<BldConfig>,
    execution: Arc<Execution>,
    logger: Arc<LoggerSender>,
    prx: Arc<PipelineFileSystemProxy>,
    pip: Pipeline,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Arc<HashMap<String, String>>,
    vars: Arc<HashMap<String, String>>,
    context: Arc<ContextSender>,
    platform: TargetPlatform,
    is_child: bool,
    has_faulted: bool,
}

impl Runner {
    async fn register_start(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting the pipeline as running in the execution context");
            self.execution.set_as_running()?;
        }
        Ok(())
    }

    async fn register_completion(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting state of root pipeline");
            if self.has_faulted {
                self.execution.set_as_faulted()?;
            } else {
                self.execution.set_as_finished()?;
            }
        }
        if self.pip.dispose {
            debug!("executing dispose operations for platform");
            self.platform.dispose(self.is_child).await?;
        } else {
            debug!("keeping platform alive");
            self.platform.keep_alive().await?;
        }
        Ok(())
    }

    fn check_stop_signal(&self) -> Result<()> {
        debug!("checking for stop signal");
        self.execution.check_stop_signal()
    }

    async fn ipc_send_completed(&self) -> Result<()> {
        if !self.is_child {
            if let Some(ipc) = Option::as_ref(&self.ipc) {
                debug!("sending message to supervisor for a completed run");
                ipc.send(WorkerMessages::Completed).await?;
            }
        }
        Ok(())
    }

    async fn info(&self) -> Result<()> {
        debug!("printing pipeline informantion");

        if let Some(name) = &self.pip.name {
            let message = format!("Pipeline: {name}");
            self.logger.write_line(message).await?;
        }

        let message = format!("Runs on: {}", self.pip.runs_on);
        self.logger.write_line(message).await?;

        Ok(())
    }

    fn apply_run_properties(&self, txt: &str) -> String {
        let mut txt_with_props = String::from(txt);
        txt_with_props = txt_with_props.replace(RUN_PROPS_ID, &self.run_id);
        txt_with_props = txt_with_props.replace(RUN_PROPS_START_TIME, &self.run_start_time);
        txt_with_props
    }

    fn apply_environment(&self, txt: &str) -> String {
        let mut txt_with_env = String::from(txt);
        for (key, value) in self.env.iter() {
            let full_name = format!("{ENV_TOKEN}{key}");
            txt_with_env = txt_with_env.replace(&full_name, value);
        }
        for env in self.pip.environment.iter() {
            let full_name = format!("{ENV_TOKEN}{}", &env.name);
            txt_with_env = txt_with_env.replace(&full_name, &env.default_value);
        }
        txt_with_env
    }

    fn apply_variables(&self, txt: &str) -> String {
        let mut txt_with_vars = String::from(txt);
        for (key, value) in self.vars.iter() {
            let full_name = format!("{VAR_TOKEN}{key}");
            txt_with_vars = txt_with_vars.replace(&full_name, value);
        }
        for variable in self.pip.variables.iter() {
            let full_name = format!("{VAR_TOKEN}{}", &variable.name);
            txt_with_vars = txt_with_vars.replace(&full_name, &variable.default_value);
        }
        txt_with_vars
    }

    fn apply_context(&self, txt: &str) -> String {
        let txt = self.apply_run_properties(txt);
        let txt = self.apply_environment(&txt);
        self.apply_variables(&txt)
    }

    async fn artifacts(&self, name: &Option<String>) -> Result<()> {
        debug!("executing artifact operation related to step {:?}", name);

        for artifact in self.pip.artifacts.iter().filter(|a| &a.after == name) {
            let can_continue = (artifact.method == Some(PUSH.to_string())
                || artifact.method == Some(GET.to_string()))
                && artifact.from.is_some()
                && artifact.to.is_some();

            if can_continue {
                debug!("applying context for artifact");

                let method = self.apply_context(artifact.method.as_ref().unwrap());
                let from = self.apply_context(artifact.from.as_ref().unwrap());
                let to = self.apply_context(artifact.to.as_ref().unwrap());
                self.logger
                    .write_line(format!(
                        "Copying artifacts from: {from} into container to: {to}",
                    ))
                    .await?;

                let result = match &method[..] {
                    PUSH => {
                        debug!("executing {PUSH} artifact operation");
                        self.platform.push(&from, &to).await
                    }
                    GET => {
                        debug!("executing {GET} artifact operation");
                        self.platform.get(&from, &to).await
                    }
                    _ => unreachable!(),
                };

                if !artifact.ignore_errors {
                    result?;
                }
            }
        }

        Ok(())
    }

    async fn steps(&mut self) -> Result<()> {
        debug!("starting execution of pipeline steps");
        for step in &self.pip.steps {
            self.step(step).await?;
            self.artifacts(&step.name).await?;
            self.check_stop_signal()?;
        }
        Ok(())
    }

    async fn step(&self, step: &BuildStep) -> Result<()> {
        if let Some(name) = &step.name {
            self.logger.write_line(format!("Step: {name}")).await?;
        }
        self.invoke(step).await?;
        self.call(step).await?;
        self.sh(step).await?;
        Ok(())
    }

    async fn invoke(&self, step: &BuildStep) -> Result<()> {
        debug!(
            "starting execution of invoke section for step {:?}",
            step.name
        );
        for invoke in &step.invoke {
            if let Some(invoke) = self.pip.invoke.iter().find(|i| &i.name == invoke) {
                let server = self.cfg.remote.server(&invoke.server)?;
                let server_auth = self.cfg.remote.same_auth_as(server)?;
                let headers = headers(&server_auth.name, &server_auth.auth)?;

                let variables = invoke
                    .variables
                    .iter()
                    .map(|e| (e.name.to_string(), self.apply_context(&e.default_value)))
                    .collect();

                let environment = invoke
                    .environment
                    .iter()
                    .map(|e| (e.name.to_string(), self.apply_context(&e.default_value)))
                    .collect();

                let url = format!(
                    "{}://{}:{}/ws-exec/",
                    server.ws_protocol(),
                    server.host,
                    server.port
                );

                debug!(
                    "establishing web socket connection with server {}",
                    server.name
                );

                let mut client = awc_client()?.ws(url);
                for (key, value) in headers.iter() {
                    client = client.header(&key[..], &value[..]);
                }

                let (_, framed) = client.connect().await.map_err(|e| anyhow!(e.to_string()))?;
                let (sink, stream) = framed.split();
                let addr = ExecClient::create(|ctx| {
                    ExecClient::add_stream(stream, ctx);
                    ExecClient::new(self.logger.clone(), SinkWrite::new(sink, ctx))
                });

                debug!("sending message for pipeline execution over the web socket");

                addr.send(RunInfo::new(
                    &invoke.pipeline,
                    Some(environment),
                    Some(variables),
                ))
                .await
                .map_err(|e| anyhow!(e))?;

                while addr.connected() {
                    sleep(Duration::from_millis(300)).await;
                }
            }
        }
        Ok(())
    }

    async fn call(&self, step: &BuildStep) -> Result<()> {
        debug!("starting execution of call section for step");
        for call in &step.call {
            let call = self.apply_context(call);

            debug!("building runner for child pipeline");

            let runner = RunnerBuilder::default()
                .run_id(&self.run_id)
                .run_start_time(&self.run_start_time)
                .config(self.cfg.clone())
                .proxy(self.prx.clone())
                .pipeline(&call)
                .execution(self.execution.clone())
                .logger(self.logger.clone())
                .environment(self.env.clone())
                .variables(self.vars.clone())
                .ipc(self.ipc.clone())
                .context(self.context.clone())
                .is_child(true)
                .build()
                .await?;

            debug!("starting child pipeline runner");

            runner.run().await.await?;
            self.check_stop_signal()?;
        }
        Ok(())
    }

    async fn sh(&self, step: &BuildStep) -> Result<()> {
        debug!("start execution of exec section for step");
        for command in step.commands.iter() {
            let working_dir = step.working_dir.as_ref().map(|wd| self.apply_context(wd));
            let command = self.apply_context(command);

            debug!("executing shell command {}", command);
            self.platform
                .shell(&working_dir, &command, self.execution.clone())
                .await?;

            self.check_stop_signal()?;
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        self.register_start().await?;
        self.info().await?;
        Ok(())
    }

    async fn execute(&mut self) -> Result<()> {
        // using let expressions to log the errors and let an empty string be used
        // by the final print_error of main.

        if let Err(e) = self.artifacts(&None).await {
            self.logger.write(e.to_string()).await?;
            self.has_faulted = true;
            bail!("");
        }

        if let Err(e) = self.steps().await {
            self.logger.write(e.to_string()).await?;
            self.has_faulted = true;
            bail!("");
        }

        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        debug!("starting cleanup operations for runner");
        self.register_completion().await?;
        self.ipc_send_completed().await?;
        Ok(())
    }

    pub async fn run(mut self) -> RecursiveFuture {
        Box::pin(async move {
            self.start().await?;
            let execution_result = self.execute().await;
            let cleanup_result = self.cleanup().await;
            debug!("runner completed");
            execution_result.and(cleanup_result)
        })
    }
}
