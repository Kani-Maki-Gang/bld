use crate::{BuildStep, Container, Machine, Pipeline, RunsOn, TargetPlatform};
use actix::{io::SinkWrite, Actor, StreamHandler};
use anyhow::{anyhow, bail, Result};
use awc::http::Version;
use awc::Client;
use bld_config::definitions::{
    ENV_TOKEN, GET, PUSH, RUN_PROPS_ID, RUN_PROPS_START_TIME, VAR_TOKEN,
};
use bld_config::BldConfig;
use bld_core::context::Context;
use bld_core::execution::Execution;
use bld_core::logger::Logger;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_sock::clients::ExecClient;
use bld_sock::messages::{RunInfo, WorkerMessages};
use bld_utils::request::headers;
use chrono::offset::Local;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::debug;
use uuid::Uuid;

type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;
type AtomicExec = Arc<Mutex<Execution>>;
type AtomicLog = Arc<Mutex<Logger>>;
type AtomicVars = Arc<HashMap<String, String>>;
type AtomicProxy = Arc<PipelineFileSystemProxy>;
type AtomicContext = Arc<Mutex<Context>>;

pub struct RunnerBuilder {
    run_id: String,
    run_start_time: String,
    cfg: Option<Arc<BldConfig>>,
    ex: AtomicExec,
    lg: AtomicLog,
    prx: AtomicProxy,
    pip: Option<String>,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Option<AtomicVars>,
    vars: Option<AtomicVars>,
    context: AtomicContext,
    is_child: bool,
}

impl Default for RunnerBuilder {
    fn default() -> Self {
        Self {
            run_id: Uuid::new_v4().to_string(),
            run_start_time: Local::now().format("%F %X").to_string(),
            cfg: None,
            ex: Execution::empty_atom(),
            lg: Logger::empty_atom(),
            prx: Arc::new(PipelineFileSystemProxy::Local),
            pip: None,
            ipc: Arc::new(None),
            env: None,
            vars: None,
            context: Arc::new(Mutex::new(Context::Empty)),
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

    pub fn execution(mut self, ex: AtomicExec) -> Self {
        self.ex = ex;
        self
    }

    pub fn logger(mut self, lg: AtomicLog) -> Self {
        self.lg = lg;
        self
    }

    pub fn pipeline(mut self, name: &str) -> Self {
        self.pip = Some(name.to_string());
        self
    }

    pub fn proxy(mut self, prx: AtomicProxy) -> Self {
        self.prx = prx;
        self
    }

    pub fn ipc(mut self, sender: Arc<Option<Sender<WorkerMessages>>>) -> Self {
        self.ipc = sender;
        self
    }

    pub fn environment(mut self, env: AtomicVars) -> Self {
        self.env = Some(env);
        self
    }

    pub fn variables(mut self, vars: AtomicVars) -> Self {
        self.vars = Some(vars);
        self
    }

    pub fn context(mut self, context: AtomicContext) -> Self {
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
        let env: Arc<HashMap<String, String>> = Arc::new(
            pipeline
                .environment
                .iter()
                .map(|e| {
                    (
                        e.name.to_string(),
                        env.get(&e.name).unwrap_or(&e.default_value).to_string(),
                    )
                })
                .collect(),
        );
        let vars = self
            .vars
            .ok_or_else(|| anyhow!("no variables instance provided"))?;
        let vars: Arc<HashMap<String, String>> = Arc::new(
            pipeline
                .variables
                .iter()
                .map(|v| {
                    (
                        v.name.to_string(),
                        vars.get(&v.name).unwrap_or(&v.default_value).to_string(),
                    )
                })
                .collect(),
        );
        let platform = match &pipeline.runs_on {
            RunsOn::Machine => {
                let machine = Machine::new(&self.run_id, env.clone(), self.lg.clone())?;
                TargetPlatform::Machine(Box::new(machine))
            }
            RunsOn::Docker(img) => {
                let container = Container::new(
                    img,
                    cfg.clone(),
                    env.clone(),
                    self.lg.clone(),
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
            ex: self.ex,
            lg: self.lg,
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
    ex: AtomicExec,
    lg: AtomicLog,
    prx: AtomicProxy,
    pip: Pipeline,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: AtomicVars,
    vars: AtomicVars,
    context: AtomicContext,
    platform: TargetPlatform,
    is_child: bool,
    has_faulted: bool,
}

impl Runner {
    fn log_dump(&self, message: &str) {
        let mut lg = self.lg.lock().unwrap();
        lg.dump(message);
    }

    async fn exec_persist_start(&self) {
        let mut exec = self.ex.lock().unwrap();
        if !self.is_child {
            debug!("setting the pipeline as running in the execution context");
            let _ = exec.set_as_running();
        }
    }

    async fn exec_persist_end(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting state of root pipeline");
            let mut exec = self.ex.lock().unwrap();
            let _ = if self.has_faulted {
                exec.set_as_faulted()
            } else {
                exec.set_as_finished()
            };
        }
        if self.pip.dispose {
            debug!("executing dispose operations for platform");
            self.platform.dispose(self.is_child).await?;
        } else {
            debug!("keeping platform alive");
            self.platform.keep_alive()?;
        }
        Ok(())
    }

    fn exec_check_stop_signal(&self) -> Result<()> {
        debug!("checking for stop signal");
        let exec = self.ex.lock().unwrap();
        exec.check_stop_signal()
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

    fn info(&self) {
        debug!("printing pipeline informantion");
        let mut logger = self.lg.lock().unwrap();
        if let Some(name) = &self.pip.name {
            logger.dumpln(&format!("[bld] Pipeline: {name}"));
        }
        logger.dumpln(&format!("[bld] Runs on: {}", self.pip.runs_on));
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
                {
                    let mut logger = self.lg.lock().unwrap();
                    logger.dumpln(&format!(
                        "[bld] Copying artifacts from: {from} into container to: {to}",
                    ));
                }
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
            self.exec_check_stop_signal()?;
        }
        Ok(())
    }

    async fn step(&self, step: &BuildStep) -> Result<()> {
        if let Some(name) = &step.name {
            let mut logger = self.lg.lock().unwrap();
            logger.infoln(&format!("[bld] Step: {name}"));
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

                let client = Client::builder()
                    .max_http_version(Version::HTTP_11)
                    .finish();
                let mut client = client.ws(url);
                for (key, value) in headers.iter() {
                    client = client.header(&key[..], &value[..]);
                }

                let (_, framed) = client.connect().await.map_err(|e| anyhow!(e.to_string()))?;
                let (sink, stream) = framed.split();
                let addr = ExecClient::create(|ctx| {
                    ExecClient::add_stream(stream, ctx);
                    ExecClient::new(self.lg.clone(), SinkWrite::new(sink, ctx))
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
                .execution(self.ex.clone())
                .logger(self.lg.clone())
                .environment(self.env.clone())
                .variables(self.vars.clone())
                .ipc(self.ipc.clone())
                .context(self.context.clone())
                .is_child(true)
                .build()
                .await?;

            debug!("starting child pipeline runner");

            runner.run().await.await?;
            self.exec_check_stop_signal()?;
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
                .shell(&working_dir, &command, self.ex.clone())
                .await?;

            self.exec_check_stop_signal()?;
        }
        Ok(())
    }

    async fn start(&self) {
        self.exec_persist_start().await;
        self.info();
    }

    async fn execute(&mut self) -> Result<()> {
        // using let expressions to log the errors and let an empty string be used
        // by the final print_error of main.

        if let Err(e) = self.artifacts(&None).await {
            self.log_dump(&e.to_string());
            self.has_faulted = true;
            bail!("");
        }

        if let Err(e) = self.steps().await {
            self.log_dump(&e.to_string());
            self.has_faulted = true;
            bail!("");
        }

        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        debug!("starting cleanup operations for runner");
        self.exec_persist_end().await?;
        self.ipc_send_completed().await?;
        Ok(())
    }

    pub async fn run(mut self) -> RecursiveFuture {
        Box::pin(async move {
            self.start().await;
            let execution_result = self.execute().await;
            let cleanup_result = self.cleanup().await;
            debug!("runner completed");
            execution_result.and(cleanup_result)
        })
    }
}
