use anyhow::Result;
use async_trait::async_trait;
use bld_config::definitions::TOOL_DIR;
use bld_config::path;
use bld_utils::fs::IsYaml;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    pipeline::traits::{ApplyTokens, CompleteTokenTransformer},
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

#[async_trait]
impl<'a> ApplyTokens<'a, PipelineContext<'a>> for BuildStep {
    async fn apply_tokens(&mut self, context: &'a PipelineContext<'a>) -> Result<()> {
        if let Some(name) = self.name.as_mut() {
            self.name = Some(
                <PipelineContext as CompleteTokenTransformer>::transform(context, name.to_owned())
                    .await?,
            );
        }

        if let Some(working_dir) = self.working_dir.as_mut() {
            self.working_dir = Some(
                <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    working_dir.to_owned(),
                )
                .await?,
            );
        }

        for exec in self.exec.iter_mut() {
            exec.apply_tokens(context).await?;
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

#[async_trait]
impl<'a> ApplyTokens<'a, PipelineContext<'a>> for BuildStepExec {
    async fn apply_tokens(&mut self, context: &'a PipelineContext<'a>) -> Result<()> {
        match self {
            Self::Shell(cmd) => {
                *cmd = <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    cmd.to_owned(),
                )
                .await?;
            }
            Self::External { value } => {
                *value = <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    value.to_owned(),
                )
                .await?;
            }
        }
        Ok(())
    }
}
