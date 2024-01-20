use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_utils::sync::IntoArc;

use crate::{
    context::ContextSender,
    logger::LoggerSender,
    platform::{
        Container, Image, Machine, PlatformSender, Ssh, SshConnectOptions, SshExecutionOptions,
    },
};

use super::ContainerOptions;

pub enum PlatformOptions<'a> {
    Container {
        image: Image<'a>,
        docker_url: Option<&'a str>,
    },
    Ssh(SshConnectOptions<'a>),
    Machine,
}

impl Default for PlatformOptions<'_> {
    fn default() -> Self {
        Self::Machine
    }
}

#[derive(Default)]
pub struct PlatformBuilder<'a> {
    run_id: Option<&'a str>,
    options: PlatformOptions<'a>,
    config: Option<Arc<BldConfig>>,
    pipeline_environment: Option<&'a HashMap<String, String>>,
    environment: Option<Arc<HashMap<String, String>>>,
    logger: Option<Arc<LoggerSender>>,
    context: Option<Arc<ContextSender>>,
}

impl<'a> PlatformBuilder<'a> {
    pub fn run_id(mut self, run_id: &'a str) -> Self {
        self.run_id = Some(run_id);
        self
    }

    pub fn options(mut self, options: PlatformOptions<'a>) -> Self {
        self.options = options;
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

    pub async fn build(self) -> Result<Arc<PlatformSender>> {
        let run_id = self
            .run_id
            .ok_or_else(|| anyhow!("no run id provided for target platform builder"))?;

        let config = self
            .config
            .ok_or_else(|| anyhow!("no config provided for target platform builder"))?;

        let pipeline_env = self.pipeline_environment.ok_or_else(|| {
            anyhow!("no pipeline environment provided for target platform builder")
        })?;

        let env = self
            .environment
            .ok_or_else(|| anyhow!("no environment provided for target platform builder"))?;

        let logger = self
            .logger
            .ok_or_else(|| anyhow!("no logger provided for target platform builder"))?;

        let context = self
            .context
            .ok_or_else(|| anyhow!("no context provided for target platform builder"))?;

        let platform = match self.options {
            PlatformOptions::Container { image, docker_url } => {
                let options = ContainerOptions {
                    config,
                    docker_url,
                    image,
                    pipeline_env,
                    env,
                    logger,
                    context,
                };
                let container = Container::new(options).await?;
                PlatformSender::container(Box::new(container))
            }

            PlatformOptions::Ssh(connect) => {
                let execution = SshExecutionOptions::new(config, pipeline_env, env);
                let ssh = Ssh::new(connect, execution).await?;
                PlatformSender::ssh(Box::new(ssh))
            }

            PlatformOptions::Machine => {
                let machine = Machine::new(run_id, config, pipeline_env, env).await?;
                PlatformSender::machine(Box::new(machine))
            }
        }
        .into_arc();

        Ok(platform)
    }
}
