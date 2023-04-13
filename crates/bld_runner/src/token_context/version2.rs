use std::{collections::HashMap, sync::Arc};

use crate::{keywords::version2::{BldDirectory, Variable, Keyword, Environment, RunId, RunStartTime}, pipeline::traits::{DynamicTokenContext, StaticTokenContext}};

#[derive(Default)]
pub struct PipelineContext<'a> {
    pub bld_directory: &'a str,
    pub variables: Arc<HashMap<String, String>>,
    pub environment: Arc<HashMap<String, String>>,
    pub run_id: &'a str,
    pub run_start_time: &'a str,
}

impl<'a> StaticTokenContext<'a, BldDirectory> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.bld_directory
    }
}

impl<'a> DynamicTokenContext<'a, Variable> for PipelineContext<'a> {
    fn retrieve(&'a self) -> HashMap<String, String> {
        let variable_token = Variable::token();
        self.variables
            .iter()
            .map(|(k, v)| (format!("{variable_token}{k}"), v.to_owned()))
            .collect()
    }
}

impl<'a> DynamicTokenContext<'a, Environment> for PipelineContext<'a> {
    fn retrieve(&'a self) -> HashMap<String, String> {
        let variable_token = Variable::token();
        self.environment
            .iter()
            .map(|(k, v)| (format!("{variable_token}{k}"), v.to_owned()))
            .collect()
    }
}

impl<'a> StaticTokenContext<'a, RunId> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_id
    }
}

impl<'a> StaticTokenContext<'a, RunStartTime> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_start_time
    }
}
