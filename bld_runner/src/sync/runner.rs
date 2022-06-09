use crate::CheckStopSignal;
use crate::{BuildStep, Container, Machine, Pipeline, RunsOn};
use anyhow::anyhow;
use bld_config::definitions::{GET, PUSH, RUN_PROPS_ID, RUN_PROPS_START_TIME, VAR_TOKEN};
use bld_config::BldConfig;
use bld_core::execution::{EmptyExec, Execution};
use bld_core::logger::Logger;
use bld_core::proxies::PipelineFileSystemProxy;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

type RecursiveFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>>>>;
type AtomicExec = Arc<Mutex<dyn Execution>>;
type AtomicLog = Arc<Mutex<dyn Logger>>;
type AtomicRecv = Arc<Mutex<Receiver<bool>>>;
type AtomicVars = Arc<HashMap<String, String>>;
type AtomicProxy = Arc<dyn PipelineFileSystemProxy>;

pub enum TargetPlatform {
    Machine(Box<Machine>),
    Container(Box<Container>),
}

#[derive(Default)]
pub struct RunnerBuilder {
    run_id: Option<String>,
    run_start_time: Option<String>,
    cfg: Option<Arc<BldConfig>>,
    ex: Option<AtomicExec>,
    lg: Option<AtomicLog>,
    prx: Option<AtomicProxy>,
    pip: Option<String>,
    cm: Option<AtomicRecv>,
    env: Option<AtomicVars>,
    vars: Option<AtomicVars>,
}

impl RunnerBuilder {
    pub fn set_run_id(mut self, id: &str) -> Self {
        self.run_id = Some(String::from(id));
        self
    }

    pub fn set_run_start_time(mut self, time: &str) -> Self {
        self.run_start_time = Some(String::from(time));
        self
    }

    pub fn set_config(mut self, cfg: Arc<BldConfig>) -> Self {
        self.cfg = Some(cfg);
        self
    }

    pub fn set_exec(mut self, ex: AtomicExec) -> Self {
        self.ex = Some(ex);
        self
    }

    pub fn set_log(mut self, lg: AtomicLog) -> Self {
        self.lg = Some(lg);
        self
    }

    pub fn set_pipeline(mut self, name: &str) -> anyhow::Result<Self> {
        self.pip = Some(name.to_string());
        Ok(self)
    }

    pub fn set_proxy(mut self, prx: AtomicProxy) -> Self {
        self.prx = Some(prx);
        self
    }

    pub fn set_receiver(mut self, cm: Option<AtomicRecv>) -> Self {
        self.cm = cm;
        self
    }

    pub fn set_environment(mut self, env: AtomicVars) -> Self {
        self.env = Some(env);
        self
    }

    pub fn set_variables(mut self, vars: AtomicVars) -> Self {
        self.vars = Some(vars);
        self
    }

    pub async fn build(self) -> anyhow::Result<Runner> {
        let id = self.run_id.ok_or(anyhow!("no run id provided"))?;
        let cfg = self.cfg.ok_or(anyhow!("no bld config instance provided"))?;
        let lg = self.lg.ok_or(anyhow!("no logger instance provided"))?;
        let prx = self
            .prx
            .ok_or(anyhow!("no pipeline file system proxy provided"))?;
        let pip_name = self.pip.ok_or(anyhow!("no pipeline provided"))?;
        let pip = Pipeline::parse(&prx.read(&pip_name)?)?;
        let platform = match &pip.runs_on {
            RunsOn::Machine => TargetPlatform::Machine(Box::new(Machine::new(&id, lg.clone())?)),
            RunsOn::Docker(img) => TargetPlatform::Container(Box::new(
                Container::new(img, cfg.clone(), lg.clone()).await?,
            )),
        };
        Ok(Runner {
            run_id: id,
            run_start_time: self
                .run_start_time
                .ok_or(anyhow!("no run start time provided"))?,
            cfg,
            ex: self.ex.ok_or(anyhow!("no executor instance provided"))?,
            lg,
            prx,
            pip,
            cm: self.cm,
            env: self.env.ok_or(anyhow!("no environment instance provided"))?,
            vars: self.vars.ok_or(anyhow!("no variables instance provided"))?,
            platform,
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
    cm: Option<AtomicRecv>,
    env: AtomicVars,
    vars: AtomicVars,
    platform: TargetPlatform,
}

impl Runner {
    fn dumpln(&self, message: &str) {
        let mut lg = self.lg.lock().unwrap();
        lg.dumpln(message);
    }

    fn persist_start(&mut self) {
        let mut exec = self.ex.lock().unwrap();
        let _ = exec.update_running(true);
    }

    fn persist_end(&mut self) {
        let mut exec = self.ex.lock().unwrap();
        let _ = exec.update_running(false);
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
                let result = match (&self.platform, &method[..]) {
                    (TargetPlatform::Container(container), PUSH) => {
                        container.copy_into(&from, &to).await
                    }
                    (TargetPlatform::Container(container), GET) => {
                        container.copy_from(&from, &to).await
                    }
                    (TargetPlatform::Machine(machine), PUSH) => machine.copy_into(&from, &to),
                    (TargetPlatform::Machine(machine), GET) => machine.copy_from(&from, &to),
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
        for step in self.pip.steps.iter() {
            self.step(step).await?;
            self.artifacts(&step.name).await?;
            self.cm.check_stop_signal()?;
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
                .set_run_id(&self.run_id)
                .set_run_start_time(&self.run_start_time)
                .set_config(self.cfg.clone())
                .set_proxy(self.prx.clone())
                .set_pipeline(call)?
                .set_exec(EmptyExec::atom())
                .set_log(self.lg.clone())
                .set_receiver(self.cm.as_ref().cloned())
                .set_environment(self.env.clone())
                .set_variables(self.vars.clone())
                .build()
                .await?;
            runner.run().await.await?;
            self.cm.check_stop_signal()?;
        }
        Ok(())
    }

    async fn sh(&self, step: &BuildStep) -> anyhow::Result<()> {
        for command in step.commands.iter() {
            let working_dir = step.working_dir.as_ref().map(|wd| self.apply_context(wd));
            let command = self.apply_context(command);
            match &self.platform {
                TargetPlatform::Container(container) => {
                    container.sh(&working_dir, &command, &self.cm).await?
                }
                TargetPlatform::Machine(machine) => machine.sh(&working_dir, &command)?,
            }
            self.cm.check_stop_signal()?;
        }
        Ok(())
    }

    async fn dispose(&self) -> anyhow::Result<()> {
        if self.pip.dispose {
            match &self.platform {
                TargetPlatform::Machine(machine) => machine.dispose()?,
                TargetPlatform::Container(container) => container.dispose().await?,
            }
        }
        Ok(())
    }

    pub async fn run(mut self) -> RecursiveFuture {
        Box::pin(async move {
            self.persist_start();
            self.info();
            match self.artifacts(&None).await {
                Ok(_) => {
                    if let Err(e) = self.steps().await {
                        self.dumpln(&e.to_string());
                    }
                }
                Err(e) => self.dumpln(&e.to_string()),
            }
            self.persist_end();
            self.dispose().await
        })
    }
}
