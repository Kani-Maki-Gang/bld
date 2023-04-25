use anyhow::Result;
use bld_config::definitions::TOOL_DIR;
use bld_config::path;
use bld_utils::fs::IsYaml;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::token_context::v2::PipelineContext;

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

    pub async fn apply_tokens<'a>(&mut self, context: &PipelineContext<'a>) -> Result<()> {
        if let Some(name) = self.name.as_mut() {
            self.name = Some(context.transform(name.to_owned()).await?);
        }

        if let Some(working_dir) = self.working_dir.as_mut() {
            self.working_dir = Some(context.transform(working_dir.to_owned()).await?);
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

impl BuildStepExec {
    pub async fn apply_tokens<'a>(&mut self, context: &PipelineContext<'a>) -> Result<()> {
        match self {
            Self::Shell(cmd) => {
                *cmd = context.transform(cmd.to_owned()).await?;
            }
            Self::External { value } => {
                *value = context.transform(value.to_owned()).await?;
            }
        }
        Ok(())
    }
}
