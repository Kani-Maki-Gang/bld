use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[cfg(feature = "all")]
use crate::token_context::v3::PipelineContext;

#[cfg(feature = "all")]
use crate::traits::Dependencies;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(feature = "all")]
impl Dependencies for BuildStep {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        match self {
            Self::One(exec) => exec.local_deps(config),
            Self::Many { exec, .. } => exec.iter().flat_map(|e| e.local_deps(config)).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(feature = "all")]
impl Dependencies for BuildStepExec {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        match self {
            BuildStepExec::External { value } if config.full_path(value).is_yaml() => {
                vec![value.to_owned()]
            }
            _ => vec![],
        }
    }
}
