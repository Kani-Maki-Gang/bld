use crate::config::BldConfig;
use crate::persist::{Execution, Logger, NullExec};
use crate::run::RunPlatform;
use crate::run::{BuildStep, Pipeline};
use crate::types::{CheckStopSignal, Result};
use std::rc::Rc;
use std::future::Future;
use std::pin::Pin;
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

    async fn steps(&mut self) -> Result<()> {
        for step in self.pip.steps.iter() {
            self.step(&step).await?;
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
            let res = runner.steps().await;
            let clean = runner.dispose().await;
            runner.persist_end();
            res.and(clean)
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
