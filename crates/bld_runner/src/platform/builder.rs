use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::{
    context::ContextSender,
    logger::LoggerSender,
    platform::{Container, Image, Machine, TargetPlatform},
};
use bld_utils::sync::IntoArc;

#[derive(Default)]
pub struct TargetPlatformBuilder<'a> {
    run_id: Option<&'a str>,
    image: Option<Image>,
    config: Option<Arc<BldConfig>>,
    pipeline_environment: Option<&'a HashMap<String, String>>,
    environment: Option<Arc<HashMap<String, String>>>,
    logger: Option<Arc<LoggerSender>>,
    context: Option<Arc<ContextSender>>,
}

impl<'a> TargetPlatformBuilder<'a> {
    pub fn run_id(mut self, run_id: &'a str) -> Self {
        self.run_id = Some(run_id);
        self
    }

    pub fn image(mut self, image: Option<Image>) -> Self {
        self.image = image;
        self
    }

    pub fn config(mut self, config: Arc<BldConfig>) -> Self {
        self.config = Some(config);
        self
    }

    pub fn pipeline_environment(mut self, environment: &'a HashMap<String, String>) -> Self {
        self.pipeline_environment = Some(environment);
        self
    }

    pub fn environment(mut self, environment: Arc<HashMap<String, String>>) -> Self {
        self.environment = Some(environment);
        self
    }

    pub fn logger(mut self, logger: Arc<LoggerSender>) -> Self {
        self.logger = Some(logger);
        self
    }

    pub fn context(mut self, context: Arc<ContextSender>) -> Self {
        self.context = Some(context);
        self
    }

    pub async fn build(self) -> Result<Arc<TargetPlatform>> {
        let run_id = self
            .run_id
            .ok_or_else(|| anyhow!("no run id provided for target platform builder"))?;

        let config = self
            .config
            .ok_or_else(|| anyhow!("no config provided for target platform builder"))?;

        let pipeline_environment = self.pipeline_environment.ok_or_else(|| {
            anyhow!("no pipeline environment provided for target platform builder")
        })?;

        let environment = self
            .environment
            .ok_or_else(|| anyhow!("no environment provided for target platform builder"))?;

        let logger = self
            .logger
            .ok_or_else(|| anyhow!("no logger provided for target platform builder"))?;

        let context = self
            .context
            .ok_or_else(|| anyhow!("no context provided for target platform builder"))?;

        let platform = match self.image {
            Some(image) => {
                let container = Container::new(
                    image,
                    config,
                    pipeline_environment,
                    environment,
                    logger,
                    context,
                )
                .await?;
                TargetPlatform::container(Box::new(container))
            }
            None => {
                let machine =
                    Machine::new(run_id, config, pipeline_environment, environment).await?;
                TargetPlatform::machine(Box::new(machine))
            }
        }
        .into_arc();

        Ok(platform)
    }
}
