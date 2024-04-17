use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[cfg(feature = "all")]
use crate::token_context::v2::PipelineContext;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildStep {
    One(BuildStepExec),
    Many {
        name: Option<String>,
        working_dir: Option<String>,
        #[serde(default)]
        exec: Vec<BuildStepExec>,
    },
}

impl BuildStep {
    #[cfg(feature = "all")]
    pub fn local_dependencies(&self, config: &BldConfig) -> Vec<String> {
        match self {
            Self::One(exec) => exec
                .local_dependencies(config)
                .map(|x| vec![x])
                .unwrap_or_default(),
            Self::Many { exec, .. } => exec
                .iter()
                .flat_map(|e| e.local_dependencies(config))
                .collect(),
        }
    }

    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &PipelineContext<'a>) -> Result<()> {
        match self {
            Self::One(exec) => {
                exec.apply_tokens(context).await?;
            }
            Self::Many {
                working_dir, exec, ..
            } => {
                if let Some(wd) = working_dir.as_mut() {
                    *working_dir = Some(context.transform(wd.to_owned()).await?);
                }
                for exec in exec.iter_mut() {
                    exec.apply_tokens(context).await?;
                }
            }
        }
        Ok(())
    }

    pub fn is(&self, name: &str) -> bool {
        let Self::Many { name: n, .. } = self else {
            return false;
        };
        n.as_ref().map(|x| x == name).unwrap_or_default()
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
    #[cfg(feature = "all")]
    pub fn local_dependencies(&self, config: &BldConfig) -> Option<String> {
        match self {
            BuildStepExec::External { value } if config.full_path(value).is_yaml() => {
                Some(value.to_owned())
            }
            _ => None,
        }
    }

    #[cfg(feature = "all")]
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
