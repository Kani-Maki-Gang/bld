use std::{fmt::Write, sync::Arc};

use anyhow::Result;
use bld_core::{logger::Logger, platform::Platform};
use tracing::debug;

use crate::{action::v3::Action, step::v3::Step};

use super::common::RecursiveFuture;

pub struct ActionRunner {
    pub logger: Arc<Logger>,
    pub action: Action,
    pub platform: Arc<Platform>,
}

impl ActionRunner {
    async fn info(&self) -> Result<()> {
        debug!("printing action informantion");

        let mut message = String::new();

        writeln!(message, "{:<15}: {}", "Name", self.action.name)?;
        writeln!(message, "{:<15}: 3", "Version")?;

        self.logger.write_line(message).await
    }

    async fn shell(&self, working_dir: &Option<String>, command: &str) -> Result<()> {
        debug!("start execution of exec section for step");
        debug!("executing shell command {}", command);
        self.platform
            .shell(self.logger.clone(), working_dir, command)
            .await?;

        Ok(())
    }

    async fn steps(&self) -> Result<()> {
        debug!("starting execution of action steps");

        for step in &self.action.steps {
            match step {
                Step::SingleSh(sh) => self.shell(&None, sh).await?,

                Step::ComplexSh(complex) => {
                    if let Some(name) = complex.name.as_ref() {
                        let mut message = String::new();
                        writeln!(message, "{:<15}: {name}", "Step")?;
                        self.logger.write_line(message).await?;
                    }
                    self.shell(&complex.working_dir, &complex.run).await?;
                }

                Step::ExternalFile(_external) => {
                    unimplemented!()
                }
            }
        }

        Ok(())
    }

    async fn execute(self) -> Result<()> {
        self.info().await?;
        self.steps().await?;
        Ok(())
    }

    pub fn run(self) -> RecursiveFuture {
        Box::pin(async move { self.execute().await })
    }
}
