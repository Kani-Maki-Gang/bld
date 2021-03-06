use crate::config::definitions::{GET, PUSH};
use crate::config::BldConfig;
use crate::persist::{Execution, Logger, NullExec};
use crate::run::RunPlatform;
use crate::run::{BuildStep, Pipeline};
use crate::types::{CheckStopSignal, Result};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;
type AtomicExec = Arc<Mutex<dyn Execution>>;
type AtomicLog = Arc<Mutex<dyn Logger>>;
type AtomicRecv = Arc<Mutex<Receiver<bool>>>;

pub struct Runner {
    pub ex: AtomicExec,
    pub lg: AtomicLog,
    pub pip: Pipeline,
    pub cm: Option<AtomicRecv>,
}

impl Runner {
    async fn new(
        cfg: Rc<BldConfig>,
        ex: AtomicExec,
        lg: AtomicLog,
        mut pip: Pipeline,
        cm: Option<AtomicRecv>,
    ) -> Result<Runner> {
        if let RunPlatform::Docker(container) = &pip.runs_on {
            pip.runs_on = RunPlatform::Docker(Box::new(container.start(cfg).await?));
        }
        Ok(Runner { ex, lg, pip, cm })
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
            logger.dumpln(&format!("Pipeline: {}", name));
        }
        logger.dumpln(&format!("Runs on: {}", self.pip.runs_on));
    }

    async fn artifacts(&self, name: &Option<String>) -> Result<()> {
        for artifact in self
            .pip
            .artifacts
            .iter()
            .filter(|a| &a.after == name)
        {
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
                        "Copying artifacts from: {} into container at: {}",
                        from, to
                    ));
                }
                match &self.pip.runs_on {
                    RunPlatform::Docker(container) => {
                        let result = if method == PUSH {
                            container.copy_into(&from, &to).await
                        } else {
                            container.copy_from(&from, &to).await
                        };
                        if !artifact.ignore_errors {
                            result?;
                        }
                    }
                    RunPlatform::Local(machine) => {
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
            logger.info(&format!("Step: {}", name));
        }
        let comm = match &self.cm {
            Some(comm) => Some(comm.clone()),
            None => None,
        };
        if let Some(call) = &step.call {
            Runner::from_file(call.clone(), NullExec::atom(), self.lg.clone(), comm)
                .await
                .await?;
        }
        self.cm.check_stop_signal()?;
        for command in step.commands.iter() {
            match &self.pip.runs_on {
                RunPlatform::Docker(container) => {
                    container.sh(&step.working_dir, &command, &self.cm).await?
                }
                RunPlatform::Local(machine) => machine.sh(&step.working_dir, &command)?,
            }
            self.cm.check_stop_signal()?;
        }
        Ok(())
    }

    async fn dispose(&self) -> Result<()> {
        if let RunPlatform::Docker(container) = &self.pip.runs_on {
            container.dispose().await?;
        }
        Ok(())
    }

    pub async fn from_src(
        src: String,
        ex: AtomicExec,
        lg: AtomicLog,
        cm: Option<AtomicRecv>,
    ) -> RecursiveFuture {
        Box::pin(async move {
            let config = Rc::new(BldConfig::load()?);
            let pip = Pipeline::parse(&src, lg.clone())?;
            let mut runner = Runner::new(Rc::clone(&config), ex, lg, pip, cm).await?;

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
    ) -> RecursiveFuture {
        Box::pin(async move {
            let src = Pipeline::read(&name)?;
            Runner::from_src(src, ex, lg, cm).await.await
        })
    }
}
