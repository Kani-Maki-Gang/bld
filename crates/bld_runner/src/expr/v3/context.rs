use super::traits::{ReadonlyRuntimeExprContext, WritableRuntimeExprContext};
use anyhow::{Result, anyhow, bail};
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

#[derive(Debug, Default)]
pub struct CommonWritableRuntimeExprContext {
    pub outputs: HashMap<String, HashMap<String, String>>,
}

impl WritableRuntimeExprContext for CommonWritableRuntimeExprContext {
    fn get_output(&self, id: &str, name: &str) -> Result<&str> {
        let Some(map) = self.outputs.get(id) else {
            bail!("id {id} has no outputs");
        };
        map.get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("output '{name}' not found"))
    }

    fn set_output(&mut self, id: String, name: String, value: String) -> Result<()> {
        if let Some(outputs) = self.outputs.get_mut(&id) {
            let _ = outputs.insert(name, value);
        } else {
            let mut map = HashMap::new();
            map.insert(name, value);
            let _ = self.outputs.insert(id, map);
        }
        Ok(())
    }
}
