use crate::persist::Dumpster;
use crate::run::Pipeline;
use crate::run::RunPlatform;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

pub struct Runner {
    pub dumpster: Arc<Mutex<dyn Dumpster>>,
    pub pipeline: Option<Pipeline>,
}

impl Runner {
    fn info(&self) {
        let mut dumpster = self.dumpster.lock().unwrap();
        let pipeline = match &self.pipeline {
            Some(p) => p,
            None => return,
        };
        if let Some(name) = &pipeline.name {
            dumpster.dumpln(&format!("Pipeline: {}", name));
        }
        dumpster.dumpln(&format!("Runs on: {}", pipeline.runs_on));
    }

    async fn steps(&mut self) -> io::Result<()> {
        let pipeline = match &self.pipeline {
            Some(p) => p,
            None => return Ok(()),
        };
        for step in pipeline.steps.iter() {
            if let Some(name) = &step.name {
                let mut dumpster = self.dumpster.lock().unwrap();
                dumpster.info(&format!("Step: {}", name));
            }

            if let Some(call) = &step.call {
                Runner::from_file(call.clone(), self.dumpster.clone())
                    .await
                    .await?;
            }

            for command in step.commands.iter() {
                match &pipeline.runs_on {
                    RunPlatform::Docker(container) => {
                        let result = container.sh(&step.working_dir, &command).await;
                        if let Err(e) = result {
                            container.dispose().await?;
                            return Err(e);
                        }
                    }
                    RunPlatform::Local(machine) => machine.sh(&step.working_dir, &command)?,
                }
            }
        }

        if let RunPlatform::Docker(container) = &pipeline.runs_on {
            container.dispose().await?;
        }

        Ok(())
    }

    pub async fn from_src(
        src: String,
        dumpster: Arc<Mutex<dyn Dumpster>>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>>>> {
        Box::pin(async move {
            let pipeline = Pipeline::parse(&src, dumpster.clone()).await?;
            let mut runner = Runner {
                dumpster,
                pipeline: Some(pipeline),
            };
            runner.info();
            runner.steps().await
        })
    }

    pub async fn from_file(
        name: String,
        dumpster: Arc<Mutex<dyn Dumpster>>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>>>> {
        Box::pin(async move {
            let src = Pipeline::read(&name)?;
            Runner::from_src(src, dumpster).await.await
        })
    }
}
