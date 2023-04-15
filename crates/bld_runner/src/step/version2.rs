use bld_config::definitions::TOOL_DIR;
use bld_config::path;
use bld_utils::fs::IsYaml;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    keywords::version2::{BldDirectory, Environment, RunId, RunStartTime, Variable},
    pipeline::traits::{
        ApplyTokens, DynamicTokenTransformer, HolisticTokenTransformer, StaticTokenTransformer,
    },
    token_context::version2::PipelineContext,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,

    #[serde(default)]
    pub exec: Vec<BuildStepExec>,
}

impl BuildStep {
    pub fn local_dependencies(&self) -> Vec<String> {
        self.exec
            .iter()
            .flat_map(|e| match e {
                BuildStepExec::External { value } if path![TOOL_DIR, value].is_yaml() => {
                    Some(value.to_owned())
                }
                _ => None,
            })
            .collect()
    }
}

impl<'a> ApplyTokens<'a, PipelineContext<'a>> for BuildStep {
    fn apply_tokens(&mut self, context: &'a PipelineContext<'a>) -> anyhow::Result<()>
    where
        Self: Sized,
        PipelineContext<'a>: StaticTokenTransformer<'a, BldDirectory>
            + DynamicTokenTransformer<'a, Variable>
            + DynamicTokenTransformer<'a, Environment>
            + StaticTokenTransformer<'a, RunId>
            + StaticTokenTransformer<'a, RunStartTime>,
    {
        self.name = self.name.as_mut().map(|x| {
            <PipelineContext as HolisticTokenTransformer>::transform(context, x.to_owned())
        });
        self.working_dir = self.working_dir.as_mut().map(|x| {
            <PipelineContext as HolisticTokenTransformer>::transform(context, x.to_owned())
        });
        for exec in self.exec.iter_mut() {
            exec.apply_tokens(context)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildStepExec {
    Shell(String),

    External {
        #[serde(rename(serialize = "ext", deserialize = "ext"))]
        value: String,
    },
}

impl<'a> ApplyTokens<'a, PipelineContext<'a>> for BuildStepExec {
    fn apply_tokens(&mut self, context: &'a PipelineContext<'a>) -> anyhow::Result<()>
    where
        Self: Sized,
        PipelineContext<'a>: StaticTokenTransformer<'a, BldDirectory>
            + DynamicTokenTransformer<'a, Variable>
            + DynamicTokenTransformer<'a, Environment>
            + StaticTokenTransformer<'a, RunId>
            + StaticTokenTransformer<'a, RunStartTime>,
    {
        match self {
            Self::Shell(cmd) => {
                *cmd = <PipelineContext as HolisticTokenTransformer>::transform(
                    context,
                    cmd.to_owned(),
                );
            }
            Self::External { value } => {
                *value = <PipelineContext as HolisticTokenTransformer>::transform(
                    context,
                    value.to_owned(),
                );
            }
        }
        Ok(())
    }
}
