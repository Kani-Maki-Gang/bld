use crate::validator::v3::{Validatable, ValidatorContext};
use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;

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
    pub async fn apply_tokens<'a>(&mut self, context: &ExecutionContext<'a>) -> Result<()> {
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

impl<'a> Validatable<'a> for BuildStep {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            BuildStep::One(exec) => {
                exec.validate(ctx).await;
            }
            BuildStep::Many {
                exec,
                working_dir,
                name,
            } => {
                if let Some(name) = name {
                    ctx.push_section(name);
                }

                if let Some(wd) = working_dir.as_ref() {
                    ctx.push_section("working_dir");
                    ctx.validate_symbols(wd);
                    ctx.pop_section();
                }

                for exec in exec.iter() {
                    exec.validate(ctx).await;
                }

                if name.is_some() {
                    ctx.pop_section();
                }
            }
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
    pub async fn apply_tokens<'a>(&mut self, context: &ExecutionContext<'a>) -> Result<()> {
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

impl<'a> Validatable<'a> for BuildStepExec {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            BuildStepExec::Shell(value) => {
                ctx.validate_symbols(value);
            }
            BuildStepExec::External { value } => {
                if ctx.contains_symbols(value) {
                    ctx.validate_symbols(value);
                } else {
                    unimplemented!()
                    // TODO: Implement this
                    // if self.pipeline.external.iter().any(|e| e.is(value)) {
                    //     return;
                    // }

                    // let fs = ctx.get_fs();
                    // let found_path = fs
                    //     .path(value)
                    //     .await
                    //     .map(|x| x.is_yaml())
                    //     .unwrap_or_default();

                    // if !found_path {
                    //     let _ = writeln!(self.errors, "[{section} > ext > {value}] Not found in either the external section or as a local pipeline");
                    // }
                }
            }
        }
    }
}
