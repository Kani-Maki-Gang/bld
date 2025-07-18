use std::{fmt::Write, sync::Arc};

use anyhow::Result;
use bld_core::{logger::Logger, platform::Platform};
use regex::Regex;
use tracing::debug;

use crate::{
    action::v3::Action,
    expr::v3::{
        context::{CommonReadonlyRuntimeExprContext, CommonWritableRuntimeExprContext},
        exec::CommonExprExecutor,
        traits::EvalExpr,
    },
    step::v3::Step,
};

use super::common::RecursiveFuture;

pub struct ActionRunner {
    pub logger: Arc<Logger>,
    pub action: Action,
    pub platform: Arc<Platform>,
    pub expr_regex: Regex,
    pub expr_rctx: CommonReadonlyRuntimeExprContext,
    pub expr_wctx: CommonWritableRuntimeExprContext,
}

impl ActionRunner {
    pub fn new(
        logger: Arc<Logger>,
        action: Action,
        platform: Arc<Platform>,
        expr_regex: Regex,
        expr_rctx: CommonReadonlyRuntimeExprContext,
    ) -> Self {
        Self {
            logger,
            action,
            platform,
            expr_regex,
            expr_rctx,
            expr_wctx: CommonWritableRuntimeExprContext::default(),
        }
    }

    async fn info(&self) -> Result<()> {
        debug!("printing action informantion");

        let mut message = String::new();

        writeln!(message, "{:<15}: {}", "Name", self.action.name)?;
        writeln!(message, "{:<15}: 3", "Version")?;

        self.logger.write_line(message).await
    }

    async fn shell(&mut self, working_dir: &Option<String>, command: &str) -> Result<()> {
        debug!("start execution of exec section for step");
        debug!("executing shell command {}", command);

        if let Some(matches) = self.expr_regex.find(command) {
            let mut command = command.to_string();
            let expr_exec =
                CommonExprExecutor::new(&self.action, &self.expr_rctx, &mut self.expr_wctx);
            let matches = matches.as_str();
            let value = expr_exec.eval(matches)?.to_string();
            command = command.replace(matches, &value);

            self.platform
                .shell(self.logger.clone(), working_dir, &command)
                .await?;
        } else {
            self.platform
                .shell(self.logger.clone(), working_dir, command)
                .await?;
        }

        Ok(())
    }

    async fn steps(&mut self) -> Result<()> {
        debug!("starting execution of action steps");
        let action = self.action.clone();
        for step in &action.steps {
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

    async fn execute(mut self) -> Result<()> {
        self.info().await?;
        self.steps().await?;
        Ok(())
    }

    pub fn run(self) -> RecursiveFuture {
        Box::pin(async move { self.execute().await })
    }
}
