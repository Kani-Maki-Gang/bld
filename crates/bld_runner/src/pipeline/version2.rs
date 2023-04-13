use crate::external::version2::External;
use crate::keywords::version2::{BldDirectory, Environment, RunId, RunStartTime, Variable};
use crate::platform::version2::Platform;
use crate::step::version2::{BuildStep, BuildStepExec};
use crate::token_context::version2::PipelineContext;
use crate::{artifacts::version2::Artifacts, keywords::version2::Keyword};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::traits::{StaticTokenTransformer, StaticTokenContext, ApplyStaticTokens, DynamicTokenContext, DynamicTokenTransformer};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: Platform,

    #[serde(default = "Pipeline::default_dispose")]
    pub dispose: bool,

    #[serde(default)]
    pub environment: HashMap<String, String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub artifacts: Vec<Artifacts>,

    #[serde(default)]
    pub external: Vec<External>,

    #[serde(default)]
    pub steps: Vec<BuildStep>,
}

impl Pipeline {
    fn default_dispose() -> bool {
        true
    }

    pub fn local_dependencies(&self) -> Vec<String> {
        let from_steps = self.steps.iter().flat_map(|s| s.local_dependencies());

        let from_external = self
            .external
            .iter()
            .filter(|e| e.server.is_none())
            .map(|e| e.pipeline.to_owned());

        from_steps.chain(from_external).collect()
    }

    fn transform_text<'b>(&self, text: &str, context: &PipelineContext<'b>) -> String {
        let text = text.to_owned();
        // let text =
        //    StaticTokenTransformer::<BldDirectory, _>::transform(self, text, &context.bld_directory);
        // let text =
        //     StaticTokenTransformer::<Variable, _>::transform(self, text, context.variables.clone());
        // let text = StaticTokenTransformer::<Environment, _>::transform(self, text, &context.environment);
        // let text = StaticTokenTransformer::<RunId, _>::transform(self, text, context.run_id);
        // let text =
        //    StaticTokenTransformer::<RunStartTime, _>::transform(self, text, &context.run_start_time);
        text
    }
}

impl<'a, V> StaticTokenTransformer<'a, BldDirectory, V> for Pipeline {
    fn transform(&self, mut text: String, context: &V) -> String
    where
        V: StaticTokenContext<'a, BldDirectory>
    {
        text.replace(BldDirectory::token(), context.retrieve())
    }
}

impl<'a, V> DynamicTokenTransformer<'a, Variable, V> for Pipeline {
    fn transform(&self, mut text: String, context: &V) -> String
    where
        V: DynamicTokenContext<'a, Variable>
    {
        // let keyword_variable: &str = Variable::token();

        // for (key, value) in context.iter() {
        //     let full_name = format!("{keyword_variable}{key}");
        //     text = text.replace(&full_name, value);
        // }

        // for (key, value) in self.variables.iter() {
        //     let full_name = format!("{keyword_variable}{key}");
        //     text = text.replace(&full_name, value);
        // }

        for (name, value) in context.retrieve() {
            text = text.replace(&name, &value);
        }

        text
    }
}

impl<'a, V> DynamicTokenTransformer<'a, Environment, V> for Pipeline {
    fn transform(&self, text: String, context: &V) -> String
    where
        V: DynamicTokenContext<'a, Environment>
    {
        // let mut text = text.to_owned();
        // let keyword_environment: &str = Environment::token();

        // for (key, value) in context.iter() {
        //     let full_name = format!("{keyword_environment}{key}");
        //     text = text.replace(&full_name, value);
        // }

        // for (key, value) in self.environment.iter() {
        //     let full_name = format!("{keyword_environment}{key}");
        //     text = text.replace(&full_name, value);
        // }

        for (name, value) in context.retrieve() {
            text = text.replace(&name, &value);
        }

        text
    }
}

impl<'a, V> StaticTokenTransformer<'a, RunId, V> for Pipeline {
    fn transform(&self, text: String, context: &V) -> String
    where
        V: StaticTokenContext<'a, RunId>
    {
        text.replace(RunId::token(), context.retrieve())
    }
}

impl<'a, V> StaticTokenTransformer<'a, RunStartTime, V> for Pipeline {
    fn transform(&self, text: String, context: &V) -> String
    where
        V: StaticTokenContext<'a, RunStartTime>
    {
        text.replace(RunStartTime::token(), context.retrieve())
    }
}

impl<'a, U> ApplyStaticTokens<'a, PipelineContext<'a>, U> for Pipeline {
    fn apply_tokens(&mut self, context: &PipelineContext<'a>) -> Result<Self>
    where
        Self: Sized,
        PipelineContext<'a>: super::traits::StaticTokenContext<'a, U>,
    {
        self.runs_on = match self.runs_on {
            Platform::ContainerByPull { image, pull } => Platform::ContainerByPull {
                image: self.transform_text(&image, &context),
                pull,
            },
            Platform::ContainerByBuild {
                name,
                tag,
                dockerfile,
            } => Platform::ContainerByBuild {
                name: self.transform_text(&name, &context),
                tag: self.transform_text(&tag, &context),
                dockerfile: self.transform_text(&dockerfile, &context),
            },
            runs_on => runs_on,
        };

        // self.dispose = self.transform_text(&self.dispose.to_string(), &context).parse::<bool>()?;

        // for artifact in self.artifacts.iter_mut() {
        //     artifact.method = self.transform_text(&artifact.method, &context);
        //     artifact.from = self.transform_text(&artifact.from, &context);
        //     artifact.to = self.transform_text(&artifact.to, &context);
        // }

        // for external in self.external.iter_mut() {
        //     external.pipeline = self.transform_text(&external.pipeline, &context);
        //     if let Some(mut name) = external.name {
        //         name = self.transform_text(&name, &context);
        //     }
        //     external.variables = external
        //         .variables
        //         .into_iter()
        //         .map(|(key, value)| (key, self.transform_text(&value, &context)))
        //         .collect();
        //     external.environment = external
        //         .environment
        //         .into_iter()
        //         .map(|(key, value)| (key, self.transform_text(&value, &context)))
        //         .collect();
        // }

        // for step in self.steps.iter_mut() {
        //     if let Some(mut name) = step.name {
        //         name = self.transform_text(&name, &context);
        //     }
        //     if let Some(mut working_dir) = step.working_dir {
        //         working_dir = self.transform_text(&working_dir, &context);
        //     }
        //     for exec in step.exec.iter_mut() {
        //         match exec {
        //             BuildStepExec::Shell(mut command) => {
        //                 command = self.transform_text(&command, &context);
        //             }
        //             BuildStepExec::External { mut value } => {
        //                 value = self.transform_text(&value, &context);
        //             }
        //         }
        //     }
        // }

        Ok(self)
    }
}
