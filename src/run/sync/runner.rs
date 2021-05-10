use crate::config::definitions::{GET, PUSH, VAR_TOKEN};
use crate::config::BldConfig;
use crate::persist::{Execution, Logger, NullExec};
use crate::run::{BuildStep, Container, Machine, Pipeline, RunsOn};
use crate::types::{CheckStopSignal, Result};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;
type AtomicExec = Arc<Mutex<dyn Execution>>;
type AtomicLog = Arc<Mutex<dyn Logger>>;
type AtomicRecv = Arc<Mutex<Receiver<bool>>>;
type AtomicVars = Arc<HashMap<String, String>>;

pub enum TargetPlatform {
    Machine(Box<Machine>),
    Container(Box<Container>),
}

pub struct Runner {
    pub ex: AtomicExec,
    pub lg: AtomicLog,
    pub pip: Pipeline,
    pub cm: Option<AtomicRecv>,
    pub vars: AtomicVars,
    pub platform: TargetPlatform,
}

impl Runner {
    async fn new(
        cfg: Rc<BldConfig>,
        ex: AtomicExec,
        lg: AtomicLog,
        pip: Pipeline,
        cm: Option<AtomicRecv>,
        vars: AtomicVars,
    ) -> Result<Runner> {
        let platform = match &pip.runs_on {
            RunsOn::Machine => TargetPlatform::Machine(Box::new(Machine::new(lg.clone())?)),
            RunsOn::Docker(img) => TargetPlatform::Container(Box::new(
                Container::new(img, cfg.clone(), lg.clone()).await?,
            )),
        };
        Ok(Runner {
            ex,
            lg,
            pip,
            cm,
            vars,
            platform,
        })
    }

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
            logger.dumpln(&format!("[bld] Pipeline: {}", name));
        }
        logger.dumpln(&format!("[bld] Runs on: {}", self.pip.runs_on));
    }

    fn apply_variables(&self, command: &str) -> String {
        let mut command_with_vars = String::from(command);
        for (key, value) in self.vars.iter() {
            let full_name = format!("{}{}", VAR_TOKEN, &key);
            command_with_vars = command_with_vars.replace(&full_name, &value);
        }
        for variable in self.pip.variables.iter() {
            let full_name = format!("{}{}", VAR_TOKEN, &variable.name);
            let value = variable
                .default_value
                .as_ref()
                .map(|d| d.to_string())
                .or_else(|| Some(String::new()))
                .unwrap();
            command_with_vars = command_with_vars.replace(&full_name, &value);
        }
        command_with_vars
    }

    async fn artifacts(&self, name: &Option<String>) -> Result<()> {
        for artifact in self.pip.artifacts.iter().filter(|a| &a.after == name) {
            let can_continue = (artifact.method == Some(PUSH.to_string())
                || artifact.method == Some(GET.to_string()))
                && artifact.from.is_some()
                && artifact.to.is_some();
            if can_continue {
                let method = artifact.method.as_ref().unwrap();
                let from = artifact.from.as_ref().unwrap();
                let to = artifact.to.as_ref().unwrap();
                {
                    let mut logger = self.lg.lock().unwrap();
                    logger.dumpln(&format!(
                        "[bld] Copying artifacts from: {} into container to: {}",
                        from, to
                    ));
                }
                match &self.platform {
                    TargetPlatform::Container(container) => {
                        let result = if method == PUSH {
                            container.copy_into(&from, &to).await
                        } else {
                            container.copy_from(&from, &to).await
                        };
                        if !artifact.ignore_errors {
                            result?;
                        }
                    }
                    TargetPlatform::Machine(machine) => {
                        let result = if method == PUSH {
                            machine.copy_into(&from, &to)
                        } else {
                            machine.copy_from(&from, &to)
                        };
                        if !artifact.ignore_errors {
                            result?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn steps(&mut self) -> Result<()> {
        for step in self.pip.steps.iter() {
            self.step(&step).await?;
            self.artifacts(&step.name).await?;
            self.cm.check_stop_signal()?;
        }
        Ok(())
    }

    async fn step(&self, step: &BuildStep) -> Result<()> {
        if let Some(name) = &step.name {
            let mut logger = self.lg.lock().unwrap();
            logger.info(&format!("[bld] Step: {}", name));
        }
        let comm = match &self.cm {
            Some(comm) => Some(comm.clone()),
            None => None,
        };
        if let Some(call) = &step.call {
            Runner::from_file(
                call.clone(),
                NullExec::atom(),
                self.lg.clone(),
                comm,
                self.vars.clone(),
            )
            .await
            .await?;
        }
        self.cm.check_stop_signal()?;
        for command in step.commands.iter() {
            let command_with_vars = self.apply_variables(&command);
            match &self.platform {
                TargetPlatform::Container(container) => {
                    container
                        .sh(&step.working_dir, &command_with_vars, &self.cm)
                        .await?
                }
                TargetPlatform::Machine(machine) => {
                    machine.sh(&step.working_dir, &command_with_vars)?
                }
            }
            self.cm.check_stop_signal()?;
        }
        Ok(())
    }

    async fn dispose(&self) -> Result<()> {
        if self.pip.dispose {
            match &self.platform {
                TargetPlatform::Machine(machine) => machine.dispose()?,
                TargetPlatform::Container(container) => container.dispose().await?,
            }
        }
        Ok(())
    }

    pub async fn from_src(
        src: String,
        ex: AtomicExec,
        lg: AtomicLog,
        cm: Option<AtomicRecv>,
        vars: AtomicVars,
    ) -> RecursiveFuture {
        Box::pin(async move {
            let config = Rc::new(BldConfig::load()?);
            let pip = Pipeline::parse(&src)?;
            let mut runner = Runner::new(Rc::clone(&config), ex, lg, pip, cm, vars).await?;

            runner.persist_start();
            runner.info();
            match runner.artifacts(&None).await {
                Ok(_) => {
                    if let Err(e) = runner.steps().await {
                        runner.dumpln(&e.to_string());
                    }
                }
                Err(e) => runner.dumpln(&e.to_string()),
            }
            runner.persist_end();
            runner.dispose().await
        })
    }

    pub async fn from_file(
        name: String,
        ex: AtomicExec,
        lg: AtomicLog,
        cm: Option<AtomicRecv>,
        vars: AtomicVars,
    ) -> RecursiveFuture {
        Box::pin(async move {
            let src = Pipeline::read(&name)?;
            Runner::from_src(src, ex, lg, cm, vars).await.await
        })
    }
}
