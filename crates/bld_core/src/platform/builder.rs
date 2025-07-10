use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use bld_utils::sync::IntoArc;
use sea_orm::DatabaseConnection;

use crate::{
    logger::Logger,
    platform::{Container, Image, Machine, Platform, Ssh, SshConnectOptions, SshExecutionOptions},
};

use super::{ContainerOptions, context::PlatformContext};

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
    pipeline_env: Option<&'a HashMap<String, String>>,
    env: Option<Arc<HashMap<String, String>>>,
    logger: Option<Arc<Logger>>,
    conn: Option<Arc<DatabaseConnection>>,
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

    pub fn pipeline_env(mut self, env: &'a HashMap<String, String>) -> Self {
        self.pipeline_env = Some(env);
        self
    }

    pub fn env(mut self, env: Arc<HashMap<String, String>>) -> Self {
        self.env = Some(env);
        self
    }

    pub fn logger(mut self, logger: Arc<Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    pub fn conn(mut self, conn: Option<Arc<DatabaseConnection>>) -> Self {
        self.conn = conn;
        self
    }

    pub async fn build(self) -> Result<Arc<Platform>> {
        let run_id = self
            .run_id
            .ok_or_else(|| anyhow!("no run id provided for target platform builder"))?;

        let config = self
            .config
            .ok_or_else(|| anyhow!("no config provided for target platform builder"))?;

        let pipeline_env = self
            .pipeline_env
            .ok_or_else(|| anyhow!("no pipeline env provided for target platform builder"))?;

        let env = self
            .env
            .ok_or_else(|| anyhow!("no env provided for target platform builder"))?;

        let logger = self
            .logger
            .ok_or_else(|| anyhow!("no logger provided for target platform builder"))?;

        let platform = match self.options {
            PlatformOptions::Container { image, docker_url } => {
                let context = PlatformContext::new(run_id, self.conn);
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
                Platform::container(Box::new(container))
            }

            PlatformOptions::Ssh(connect) => {
                let execution = SshExecutionOptions::new(config, pipeline_env, env);
                let ssh = Ssh::new(connect, execution).await?;
                Platform::ssh(Box::new(ssh))
            }

            PlatformOptions::Machine => {
                let machine = Machine::new(run_id, config, pipeline_env, env).await?;
                Platform::machine(Box::new(machine))
            }
        }
        .into_arc();

        Ok(platform)
    }
}
