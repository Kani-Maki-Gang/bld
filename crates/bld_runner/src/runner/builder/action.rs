use std::{collections::HashMap, sync::Arc};

use crate::{
    action::v3,
    runner::{self, versioned::VersionedActionRunner},
    token_context::v3::ExecutionContextBuilder,
};
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::{logger::Logger, platform::Platform, regex::RegexCache};

pub enum ActionInstance {
    V3(v3::Action),
}

#[derive(Default)]
pub struct ActionRunnerBuilder<'a> {
    run_id: Option<&'a str>,
    run_start_time: Option<&'a str>,
    config: Option<Arc<BldConfig>>,
    logger: Option<Arc<Logger>>,
    action: Option<ActionInstance>,
    inputs: Option<&'a HashMap<String, String>>,
    env: Option<&'a HashMap<String, String>>,
    platform: Option<Arc<Platform>>,
    regex_cache: Option<Arc<RegexCache>>,
}

impl<'a> ActionRunnerBuilder<'a> {
    pub fn run_id(mut self, run_id: &'a str) -> Self {
        self.run_id = Some(run_id);
        self
    }

    pub fn run_start_time(mut self, run_start_time: &'a str) -> Self {
        self.run_start_time = Some(run_start_time);
        self
    }

    pub fn config(mut self, config: Arc<BldConfig>) -> Self {
        self.config = Some(config);
        self
    }

    pub fn logger(mut self, logger: Arc<Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    pub fn action(mut self, action: ActionInstance) -> Self {
        self.action = Some(action);
        self
    }

    pub fn inputs(mut self, inputs: &'a HashMap<String, String>) -> Self {
        self.inputs = Some(inputs);
        self
    }

    pub fn env(mut self, env: &'a HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }

    pub fn platform(mut self, platform: Arc<Platform>) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn regex_cache(mut self, regex_cache: Arc<RegexCache>) -> Self {
        self.regex_cache = Some(regex_cache);
        self
    }

    pub async fn build(self) -> Result<VersionedActionRunner> {
        let run_id = self.run_id.ok_or_else(|| anyhow!("no run id provided"))?;
        let run_start_time = self
            .run_start_time
            .ok_or_else(|| anyhow!("no run start time provided"))?;
        let config = self
            .config
            .ok_or_else(|| anyhow!("no bld config instance provided"))?;
        let logger = self.logger.ok_or_else(|| anyhow!("no logger provided"))?;
        let action = self.action.ok_or_else(|| anyhow!("no action provided"))?;
        let inputs = self.inputs.ok_or_else(|| anyhow!("no inputs provided"))?;
        let env = self.env.ok_or_else(|| anyhow!("no env provided"))?;
        let platform = self
            .platform
            .ok_or_else(|| anyhow!("no platform provided"))?;
        let regex_cache = self
            .regex_cache
            .ok_or_else(|| anyhow!("no regex cache provided"))?;

        match action {
            ActionInstance::V3(mut action) => {
                let execution_context = ExecutionContextBuilder::default()
                    .root_dir(&config.root_dir)
                    .project_dir(&config.project_dir)
                    .add_inputs(&action.inputs_map())
                    .add_inputs(&inputs)
                    .add_env(env)
                    .run_id(run_id)
                    .run_start_time(run_start_time)
                    .regex_cache(regex_cache.clone())
                    .build()?;

                action.apply_tokens(&execution_context).await?;

                let runner = VersionedActionRunner::V3(runner::v3::ActionRunner {
                    logger,
                    action,
                    platform,
                });
                Ok(runner)
            }
        }
    }
}
