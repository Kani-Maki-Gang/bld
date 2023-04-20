use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::{
    keywords::version2::{BldDirectory, Environment, Keyword, RunId, RunStartTime, Variable},
    pipeline::traits::{
        DynamicTokenContext, DynamicTokenTransformer, HolisticTokenTransformer, StaticTokenContext,
        StaticTokenTransformer,
    },
};

pub struct PipelineContextBuilder<'a> {
    bld_directory: Option<&'a str>,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
    run_id: Option<&'a str>,
    run_start_time: Option<&'a str>,
}

impl<'a> Default for PipelineContextBuilder<'a> {
    fn default() -> Self {
        Self {
            bld_directory: None,
            variables: HashMap::new(),
            environment: HashMap::new(),
            run_id: None,
            run_start_time: None,
        }
    }
}

impl<'a> PipelineContextBuilder<'a> {
    pub fn bld_directory(mut self, directory: &'a str) -> Self {
        self.bld_directory = Some(directory);
        self
    }

    pub fn add_variables(mut self, variables: &HashMap<String, String>) -> Self {
        let variable_token = Variable::token();
        for (k, v) in variables.iter() {
            self.variables.insert(format!("{variable_token}{k}"), v.to_owned());
        }
        self
    }

    pub fn add_environment(mut self, environment: &HashMap<String, String>) -> Self {
        let environment_token = Environment::token();
        for (k, v) in environment.iter() {
            self.environment.insert(format!("{environment_token}{k}"), v.to_owned());
        }
        self
    }

    pub fn run_id(mut self, run_id: &'a str) -> Self {
        self.run_id = Some(run_id);
        self
    }

    pub fn run_start_time(mut self, run_start_time: &'a str) -> Self {
        self.run_start_time = Some(run_start_time);
        self
    }

    pub fn build(self) -> Result<PipelineContext<'a>> {
        let bld_directory = self.bld_directory.ok_or_else(|| anyhow!("bld directory not provided in pipeline context"))?;
        let run_id = self.run_id.ok_or_else(|| anyhow!("run id not provided in pipeline context"))?;
        let run_start_time = self.run_start_time.ok_or_else(|| anyhow!("run start time not provided in pipeline context"))?;

        Ok(PipelineContext {
            bld_directory,
            variables: self.variables,
            environment: self.environment,
            run_id,
            run_start_time
        })
    }
}

pub struct PipelineContext<'a> {
    pub bld_directory: &'a str,
    pub variables: HashMap<String, String>,
    pub environment: HashMap<String, String>,
    pub run_id: &'a str,
    pub run_start_time: &'a str,
}

impl<'a> StaticTokenContext<'a, BldDirectory> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.bld_directory
    }
}

impl<'a> StaticTokenTransformer<'a, BldDirectory> for PipelineContext<'a> {
    fn transform(&'a self, text: String) -> String {
        text.replace(
            BldDirectory::token(),
            StaticTokenContext::<'a, BldDirectory>::retrieve(self),
        )
    }
}

impl<'a> DynamicTokenContext<'a, Variable> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &HashMap<String, String> {
        &self.variables
    }
}

impl<'a> DynamicTokenTransformer<'a, Variable> for PipelineContext<'a> {
    fn transform(&'a self, mut text: String) -> String {
        for (k, v) in DynamicTokenContext::<'a, Variable>::retrieve(self).iter() {
            text = text.replace(k, v);
        }
        text
    }
}

impl<'a> DynamicTokenContext<'a, Environment> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &HashMap<String, String> {
        &self.environment
    }
}

impl<'a> DynamicTokenTransformer<'a, Environment> for PipelineContext<'a> {
    fn transform(&'a self, mut text: String) -> String {
        for (k, v) in DynamicTokenContext::<'a, Environment>::retrieve(self).iter() {
            text = text.replace(k, v);
        }
        text
    }
}

impl<'a> StaticTokenContext<'a, RunId> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_id
    }
}

impl<'a> StaticTokenTransformer<'a, RunId> for PipelineContext<'a> {
    fn transform(&'a self, text: String) -> String {
        text.replace(
            RunId::token(),
            StaticTokenContext::<'a, RunId>::retrieve(self),
        )
    }
}

impl<'a> StaticTokenContext<'a, RunStartTime> for PipelineContext<'a> {
    fn retrieve(&'a self) -> &'a str {
        self.run_start_time
    }
}

impl<'a> StaticTokenTransformer<'a, RunStartTime> for PipelineContext<'a> {
    fn transform(&'a self, text: String) -> String {
        text.replace(
            RunStartTime::token(),
            StaticTokenContext::<'a, RunStartTime>::retrieve(self),
        )
    }
}

impl<'a> HolisticTokenTransformer<'a> for PipelineContext<'a> {
    fn transform(&'a self, mut text: String) -> String {
        text = <PipelineContext as StaticTokenTransformer<'a, BldDirectory>>::transform(self, text);
        text = <PipelineContext as DynamicTokenTransformer<'a, Variable>>::transform(self, text);
        text = <PipelineContext as DynamicTokenTransformer<'a, Environment>>::transform(self, text);
        text = <PipelineContext as StaticTokenTransformer<'a, RunId>>::transform(self, text);
        text = <PipelineContext as StaticTokenTransformer<'a, RunStartTime>>::transform(self, text);
        text
    }
}
