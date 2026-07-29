use super::traits::{ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext};
use anyhow::{Error, Result, anyhow};
use bld_config::BldConfig;
use bld_config::definitions::{
    KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
    KEYWORD_RUN_PROPS_START_TIME_V3,
};
use std::{collections::HashMap, sync::Arc};

pub struct StartOfRunWritableExprContext;

pub static START_OF_RUN_WCTX: StartOfRunWritableExprContext = StartOfRunWritableExprContext;

pub fn out_of_scope(symbol: &str) -> Error {
    anyhow!(
        "'{symbol}' is not available at the start of a run, only inputs, env, {KEYWORD_BLD_DIR_V3}, {KEYWORD_PROJECT_DIR_V3}, {KEYWORD_RUN_PROPS_ID_V3} and {KEYWORD_RUN_PROPS_START_TIME_V3} can be used here"
    )
}

impl WritableRuntimeExprContext for StartOfRunWritableExprContext {
    fn get_exec_id(&self) -> Option<&str> {
        None
    }

    fn get_output<'a>(&'a self, id: &str, name: &str) -> Result<ExprValue<'a>> {
        Err(out_of_scope(&format!("steps.{id}.outputs.{name}")))
    }

    fn set_output(&mut self, _id: &str, name: String, _value: String) -> Result<()> {
        Err(out_of_scope(&format!("outputs.{name}")))
    }

    fn set_outputs(&mut self, _id: &str, _outputs: HashMap<String, String>) -> Result<()> {
        Err(out_of_scope("outputs"))
    }

    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<&'a str> {
        Err(out_of_scope(&format!("matrix.{name}")))
    }
}

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
