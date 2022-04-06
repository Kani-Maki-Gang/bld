use crate::config::definitions::{GET, PUSH, VAR_TOKEN};
use crate::config::BldConfig;
use crate::persist::{EmptyExec, Execution, Logger};
use crate::run::CheckStopSignal;
use crate::run::{BuildStep, Container, Machine, Pipeline, RunsOn};
use anyhow::anyhow;
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

pub enum TargetPlatform {
    Machine(Box<Machine>),
    Container(Box<Container>),
}

#[derive(Default)]
pub struct RunnerBuilder {
    cfg: Option<Arc<BldConfig>>,
    ex: Option<AtomicExec>,
    lg: Option<AtomicLog>,
    pip: Option<Pipeline>,
    cm: Option<AtomicRecv>,
    vars: Option<AtomicVars>,
}

impl RunnerBuilder {
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

    pub fn set_from_src(mut self, src: &str) -> anyhow::Result<Self> {
        self.pip = Some(Pipeline::parse(src)?);
        Ok(self)
    }

    pub fn set_from_file(self, file: &str) -> anyhow::Result<Self> {
        self.set_from_src(&Pipeline::read(file)?)
    }

    pub fn set_receiver(mut self, cm: Option<AtomicRecv>) -> Self {
        self.cm = cm;
        self
    }

    pub fn set_variables(mut self, vars: AtomicVars) -> Self {
        self.vars = Some(vars);
        self
    }

    pub async fn build(self) -> anyhow::Result<Runner> {
        let cfg = self.cfg.ok_or(anyhow!("no bld config instance provided"))?;
        let lg = self.lg.ok_or(anyhow!("no logger instance provided"))?;
        let pip = self.pip.ok_or(anyhow!("no pipeline provided"))?;
        let platform = match &pip.runs_on {
            RunsOn::Machine => TargetPlatform::Machine(Box::new(Machine::new(lg.clone())?)),
            RunsOn::Docker(img) => TargetPlatform::Container(Box::new(
                Container::new(img, cfg.clone(), lg.clone()).await?,
            )),
        };
        Ok(Runner {
            cfg,
            ex: self.ex.ok_or(anyhow!("no executor instance provided"))?,
            lg,
            pip,
            cm: self.cm,
            vars: self.vars.ok_or(anyhow!("no variables instance provided"))?,
            platform
        })
    }
}

pub struct Runner {
    cfg: Arc<BldConfig>,
    ex: AtomicExec,
    lg: AtomicLog,
    pip: Pipeline,
    cm: Option<AtomicRecv>,
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
        let _ = exec.update(true);
    }

    fn persist_end(&mut self) {
        let mut exec = self.ex.lock().unwrap();
        let _ = exec.update(false);
    }

    fn info(&self) {
        let mut logger = self.lg.lock().unwrap();
        if let Some(name) = &self.pip.name {
            logger.dumpln(&format!("[bld] Pipeline: {name}"));
        }
        logger.dumpln(&format!("[bld] Runs on: {}", self.pip.runs_on));
    }

    fn apply_variables(&self, txt: &str) -> String {
        let mut txt_with_vars = String::from(txt);
        for (key, value) in self.vars.iter() {
            let full_name = format!("{VAR_TOKEN}{key}");
            txt_with_vars = txt_with_vars.replace(&full_name, value);
        }
        for variable in self.pip.variables.iter() {
            let full_name = format!("{VAR_TOKEN}{}", &variable.name);
            let value = variable
                .default_value
                .as_ref()
                .map(|d| d.to_string())
                .or_else(|| Some(String::new()))
                .unwrap();
            txt_with_vars = txt_with_vars.replace(&full_name, &value);
        }
        txt_with_vars
    }

    async fn artifacts(&self, name: &Option<String>) -> anyhow::Result<()> {
        for artifact in self.pip.artifacts.iter().filter(|a| &a.after == name) {
            let can_continue = (artifact.method == Some(PUSH.to_string())
                || artifact.method == Some(GET.to_string()))
                && artifact.from.is_some()
                && artifact.to.is_some();
            if can_continue {
                let method = self.apply_variables(artifact.method.as_ref().unwrap());
                let from = self.apply_variables(artifact.from.as_ref().unwrap());
                let to = self.apply_variables(artifact.to.as_ref().unwrap());
                {
                    let mut logger = self.lg.lock().unwrap();
                    logger.dumpln(&format!(
                        "[bld] Copying artifacts from: {from} into container to: {to}",
                    ));
                }
                let result = match (&self.platform, &method[..]) {
                    (TargetPlatform::Container(container), PUSH) => container.copy_into(&from, &to).await,
                    (TargetPlatform::Container(container), GET) => container.copy_from(&from, &to).await,
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
        if let Some(call) = &step.call {
            let runner = RunnerBuilder::default()
                .set_config(self.cfg.clone())
                .set_from_file(call)?
                .set_exec(EmptyExec::atom())
                .set_log(self.lg.clone())
                .set_receiver(self.cm.as_ref().cloned())
                .set_variables(self.vars.clone())
                .build()
                .await?;
            runner.run().await.await?;
        }
        self.cm.check_stop_signal()?;   
        Ok(())
    }
    
    async fn sh(&self, step: &BuildStep) -> anyhow::Result<()> {
        for command in step.commands.iter() {
            let working_dir = step.working_dir.as_ref().map(|wd| self.apply_variables(&wd));
            let command = self.apply_variables(command);
            match &self.platform {
                TargetPlatform::Container(container) => {
                    container
                        .sh(&working_dir, &command, &self.cm)
                        .await?
                }
                TargetPlatform::Machine(machine) => {
                    machine.sh(&step.working_dir, &command)?
                }
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
