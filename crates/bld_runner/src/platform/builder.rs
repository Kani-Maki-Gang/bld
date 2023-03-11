use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::{
    context::ContextSender,
    logger::LoggerSender,
    platform::{Container, Machine, TargetPlatform},
};
use bld_utils::sync::IntoArc;

use crate::{
    pipeline::{version1, version2},
    VersionedPipeline,
};

use super::version2::Platform;

#[derive(Default)]
pub struct TargetPlatformBuilder<'a> {
    run_id: Option<&'a str>,
    pipeline: Option<&'a VersionedPipeline>,
    config: Option<Arc<BldConfig>>,
    environment: Option<Arc<HashMap<String, String>>>,
    logger: Option<Arc<LoggerSender>>,
    context: Option<Arc<ContextSender>>,
}

impl<'a> TargetPlatformBuilder<'a> {
    pub fn run_id(mut self, run_id: &'a str) -> Self {
        self.run_id = Some(run_id);
        self
    }

    pub fn pipeline(mut self, pipeline: &'a VersionedPipeline) -> Self {
        self.pipeline = Some(pipeline);
        self
    }

    pub fn config(mut self, config: Arc<BldConfig>) -> Self {
        self.config = Some(config);
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

        let pipeline = self
            .pipeline
            .ok_or_else(|| anyhow!("no pipeline provided for target platform builder"))?;

        let config = self
            .config
            .ok_or_else(|| anyhow!("no config provided for target platform builder"))?;

        let environment = self
            .environment
            .ok_or_else(|| anyhow!("no environment provided for target platform builder"))?;

        let logger = self
            .logger
            .ok_or_else(|| anyhow!("no logger provided for target platform builder"))?;

        let context = self
            .context
            .ok_or_else(|| anyhow!("no context provided for target platform builder"))?;

        let platform = match pipeline {
            VersionedPipeline::Version1(version1::Pipeline { runs_on, .. })
                if runs_on == "machine" =>
            {
                let machine = Machine::new(run_id, environment, logger)?;
                TargetPlatform::machine(Box::new(machine))
            }

            VersionedPipeline::Version1(version1::Pipeline { runs_on, .. }) => {
                let container =
                    Container::new(runs_on, config, environment, logger, context).await?;
                TargetPlatform::container(Box::new(container))
            }

            VersionedPipeline::Version2(version2::Pipeline {
                runs_on: Platform::Machine,
                ..
            }) => {
                let machine = Machine::new(run_id, environment, logger)?;
                TargetPlatform::machine(Box::new(machine))
            }

            VersionedPipeline::Version2(version2::Pipeline {
                runs_on: Platform::Image(runs_on),
                ..
            }) => {
                let container =
                    Container::new(runs_on, config, environment, logger, context).await?;
                TargetPlatform::container(Box::new(container))
            }

            VersionedPipeline::Version2(version2::Pipeline {
                runs_on: Platform::Dockerfile { image, .. },
                ..
            }) => {
                let container =
                    Container::new(image, config, environment, logger, context).await?;
                TargetPlatform::container(Box::new(container))
            }
        }
        .into_arc();

        Ok(platform)
    }
}
