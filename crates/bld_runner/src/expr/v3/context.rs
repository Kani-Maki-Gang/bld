use crate::expr::v3::traits::WritableRuntimeExprContext;

use super::traits::ReadonlyRuntimeExprContext;
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Default)]
pub struct CommonReadonlyRuntimeExprContext {
    pub config: Arc<BldConfig>,
    pub inputs: Arc<HashMap<String, String>>,
    pub env: Arc<HashMap<String, String>>,
    pub run_id: String,
    pub run_start_time: String,
}

impl CommonReadonlyRuntimeExprContext {
    pub fn new(
        config: Arc<BldConfig>,
        inputs: Arc<HashMap<String, String>>,
        env: Arc<HashMap<String, String>>,
        run_id: String,
        run_start_time: String,
    ) -> Self {
        Self {
            config,
            inputs,
            env,
            run_id,
            run_start_time,
        }
    }
}

impl<'a> ReadonlyRuntimeExprContext<'a> for CommonReadonlyRuntimeExprContext {
    fn get_root_dir(&'a self) -> &'a str {
        &self.config.root_dir
    }

    fn get_project_dir(&'a self) -> &'a str {
        &self.config.project_dir
    }

    fn get_input(&'a self, name: &'a str) -> Result<&'a str> {
        self.inputs
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("input '{name}' not found"))
    }

    fn get_env(&'a self, name: &'a str) -> Result<&'a str> {
        self.env
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("env variable '{name}' not found"))
    }

    fn get_run_id(&'a self) -> &'a str {
        &self.run_id
    }

    fn get_run_start_time(&'a self) -> &'a str {
        &self.run_start_time
    }
}

#[derive(Default)]
pub struct CommonWritableRuntimeExprContext {
    outputs: HashMap<String, String>,
}

impl WritableRuntimeExprContext for CommonWritableRuntimeExprContext {
    fn get_exec_id<'a>(&'a self) -> Option<&'a str> {
        None
    }

    fn get_output<'a>(&'a self, _id: &str, name: &str) -> Result<&'a str> {
        self.outputs
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("output value not found"))
    }

    fn set_output(&mut self, _id: &str, name: String, value: String) -> Result<()> {
        self.outputs.insert(name, value);
        Ok(())
    }

    fn set_outputs(&mut self, _id: &str, outputs: HashMap<String, String>) -> Result<()> {
        self.outputs = outputs;
        Ok(())
    }
}
