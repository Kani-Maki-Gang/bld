use crate::{BuildStep, Container, Machine, Pipeline, RunsOn, TargetPlatform};
use anyhow::anyhow;
use bld_config::{
    definitions::{ENV_TOKEN, GET, PUSH, RUN_PROPS_ID, RUN_PROPS_START_TIME, VAR_TOKEN},
    BldConfig,
};
use bld_core::{
    context::Context, execution::Execution, logger::Logger, proxies::PipelineFileSystemProxy,
};
use bld_supervisor::base::WorkerMessages;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::Sender;

type RecursiveFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>>>>;
type AtomicExec = Arc<Mutex<Execution>>;
type AtomicLog = Arc<Mutex<Logger>>;
type AtomicVars = Arc<HashMap<String, String>>;
type AtomicProxy = Arc<PipelineFileSystemProxy>;
type AtomicContext = Arc<Mutex<Context>>;

#[derive(Default)]
pub struct RunnerBuilder {
    run_id: Option<String>,
    run_start_time: Option<String>,
    cfg: Option<Arc<BldConfig>>,
    ex: Option<AtomicExec>,
    lg: Option<AtomicLog>,
    prx: Option<AtomicProxy>,
    pip: Option<String>,
    ipc: Arc<Option<Sender<WorkerMessages>>>,
    env: Option<AtomicVars>,
    vars: Option<AtomicVars>,
    context: Option<AtomicContext>,
    is_child: bool,
}

impl RunnerBuilder {
    pub fn run_id(mut self, id: &str) -> Self {
        self.run_id = Some(String::from(id));
        self
    }

    pub fn run_start_time(mut self, time: &str) -> Self {
        self.run_start_time = Some(String::from(time));
        self
    }

    pub fn config(mut self, cfg: Arc<BldConfig>) -> Self {
        self.cfg = Some(cfg);
        self
    }

    pub fn execution(mut self, ex: AtomicExec) -> Self {
        self.ex = Some(ex);
        self
    }

    pub fn logger(mut self, lg: AtomicLog) -> Self {
        self.lg = Some(lg);
        self
    }

    pub fn pipeline(mut self, name: &str) -> Self {
        self.pip = Some(name.to_string());
        self
    }

    pub fn proxy(mut self, prx: AtomicProxy) -> Self {
        self.prx = Some(prx);
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
        self.context = Some(context);
        self
    }

    pub fn is_child(mut self, is_child: bool) -> Self {
        self.is_child = is_child;
        self
    }

    pub async fn build(self) -> anyhow::Result<Runner> {
        let id = self.run_id.ok_or_else(|| anyhow!("no run id provided"))?;
        let cfg = self
            .cfg
            .ok_or_else(|| anyhow!("no bld config instance provided"))?;
        let lg = self
            .lg
            .ok_or_else(|| anyhow!("no logger instance provided"))?;
        let prx = self
            .prx
            .ok_or_else(|| anyhow!("no pipeline file system proxy provided"))?;
        let pip_name = self.pip.ok_or_else(|| anyhow!("no pipeline provided"))?;
        let pipeline = Pipeline::parse(&prx.read(&pip_name)?)?;
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
        let context = self
            .context
            .ok_or_else(|| anyhow!("no container handler was provided"))?;
        let platform = match &pipeline.runs_on {
            RunsOn::Machine => {
                let machine = Machine::new(&id, env.clone(), lg.clone())?;
                TargetPlatform::Machine(Box::new(machine))
            }
            RunsOn::Docker(img) => {
                let container = Container::new(
                    img,
                    cfg.clone(),
                    env.clone(),
                    lg.clone(),
                    context.clone(),
                )
                .await?;
                TargetPlatform::Container(Box::new(container))
            }
        };
        Ok(Runner {
            run_id: id,
            run_start_time: self
                .run_start_time
                .ok_or_else(|| anyhow!("no run start time provided"))?,
            cfg,
            ex: self
                .ex
                .ok_or_else(|| anyhow!("no executor instance provided"))?,
            lg,
            prx,
            pip: pipeline,
            ipc: self.ipc,
            env,
            vars,
            context,
            platform,
            is_child: self.is_child,
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
}

impl Runner {
    fn log_dumpln(&self, message: &str) {
        let mut lg = self.lg.lock().unwrap();
        lg.dumpln(message);
    }

    async fn exec_persist_start(&self) {
        let mut exec = self.ex.lock().unwrap();
        if !self.is_child {
            let _ = exec.update_state("running");
        }
    }

    async fn exec_persist_end(&self) -> anyhow::Result<()> {
        let mut exec = self.ex.lock().unwrap();
        if !self.is_child {
            let _ = exec.update_state("finished");
        }
        if self.pip.dispose {
            self.platform.dispose(self.is_child).await?;
        }
        Ok(())
    }

    fn exec_check_stop_signal(&self) -> anyhow::Result<()> {
        let exec = self.ex.lock().unwrap();
        exec.check_stop_signal()
    }

    async fn ipc_send_completed(&self) -> anyhow::Result<()> {
        if !self.is_child {
            if let Some(ipc) = Option::as_ref(&self.ipc) {
                ipc.send(WorkerMessages::Completed).await?;
            }
        }
        Ok(())
    }

    fn info(&self) {
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

    async fn artifacts(&self, name: &Option<String>) -> anyhow::Result<()> {
        for artifact in self.pip.artifacts.iter().filter(|a| &a.after == name) {
            let can_continue = (artifact.method == Some(PUSH.to_string())
                || artifact.method == Some(GET.to_string()))
                && artifact.from.is_some()
                && artifact.to.is_some();
            if can_continue {
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
                    PUSH => self.platform.push(&from, &to).await,
                    GET => self.platform.get(&from, &to).await,
                    _ => unreachable!(),
                };
                if !artifact.ignore_errors {
                    result?;
                }
            }
        }
        Ok(())
    }

    async fn steps(&mut self) -> anyhow::Result<()> {
        for step in &self.pip.steps {
            self.step(step).await?;
            self.artifacts(&step.name).await?;
            self.exec_check_stop_signal()?;
        }
        Ok(())
    }

    async fn step(&self, step: &BuildStep) -> anyhow::Result<()> {
        if let Some(name) = &step.name {
            let mut logger = self.lg.lock().unwrap();
            logger.info(&format!("[bld] Step: {name}"));
        }
        self.call(step).await?;
        self.sh(step).await?;
        Ok(())
    }

    async fn call(&self, step: &BuildStep) -> anyhow::Result<()> {
        for call in &step.call {
            let runner = RunnerBuilder::default()
                .run_id(&self.run_id)
                .run_start_time(&self.run_start_time)
                .config(self.cfg.clone())
                .proxy(self.prx.clone())
                .pipeline(call)
                .execution(self.ex.clone())
                .logger(self.lg.clone())
                .environment(self.env.clone())
                .variables(self.vars.clone())
                .ipc(self.ipc.clone())
                .context(self.context.clone())
                .is_child(true)
                .build()
                .await?;
            runner.run().await.await?;
            self.exec_check_stop_signal()?;
        }
        Ok(())
    }

    async fn sh(&self, step: &BuildStep) -> anyhow::Result<()> {
        for command in step.commands.iter() {
            let working_dir = step.working_dir.as_ref().map(|wd| self.apply_context(wd));
            let command = self.apply_context(command);
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

    async fn execute(&mut self) {
        match self.artifacts(&None).await {
            Ok(_) => {
                if let Err(e) = self.steps().await {
                    self.log_dumpln(&e.to_string());
                }
            }
            Err(e) => self.log_dumpln(&e.to_string()),
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        self.exec_persist_end().await?;
        self.ipc_send_completed().await?;
        Ok(())
    }

    pub async fn run(mut self) -> RecursiveFuture {
        Box::pin(async move {
            self.start().await;
            self.execute().await;
            self.cleanup().await?;
            Ok(())
        })
    }
}
