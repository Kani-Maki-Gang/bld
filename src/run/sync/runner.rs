use crate::persist::Logger;
use crate::run::Pipeline;
use crate::run::RunPlatform;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

pub struct Runner {
    pub logger: Arc<Mutex<dyn Logger>>,
    pub pipeline: Option<Pipeline>,
}

impl Runner {
    fn info(&self) {
        let mut logger = self.logger.lock().unwrap();
        let pipeline = match &self.pipeline {
            Some(p) => p,
            None => return,
        };
        if let Some(name) = &pipeline.name {
            logger.dumpln(&format!("Pipeline: {}", name));
        }
        logger.dumpln(&format!("Runs on: {}", pipeline.runs_on));
    }

    async fn steps(&mut self) -> io::Result<()> {
        let pipeline = match &self.pipeline {
            Some(p) => p,
            None => return Ok(()),
        };
        for step in pipeline.steps.iter() {
            if let Some(name) = &step.name {
                let mut logger = self.logger.lock().unwrap();
                logger.info(&format!("Step: {}", name));
            }

            if let Some(call) = &step.call {
                Runner::from_file(call.clone(), self.logger.clone())
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
        logger: Arc<Mutex<dyn Logger>>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>>>> {
        Box::pin(async move {
            let pipeline = Pipeline::parse(&src, logger.clone()).await?;
            let mut runner = Runner {
                logger,
                pipeline: Some(pipeline),
            };
            runner.info();
            runner.steps().await
        })
    }

    pub async fn from_file(
        name: String,
        logger: Arc<Mutex<dyn Logger>>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>>>> {
        Box::pin(async move {
            let src = Pipeline::read(&name)?;
            Runner::from_src(src, logger).await.await
        })
    }
}
