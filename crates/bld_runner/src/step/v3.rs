use crate::validator::v3::{Validate, ValidatorContext};
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
pub struct ShellCommand {
    pub name: Option<String>,
    pub working_dir: Option<String>,
    #[serde(default)]
    pub run: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCall {
    pub name: Option<String>,
    pub working_dir: Option<String>,
    #[serde(default)]
    pub ext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
    SingleSh(String),
    ComplexSh(Box<ShellCommand>),
    External(Box<ExternalCall>),
}

impl Step {
    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &ExecutionContext<'a>) -> Result<()> {
        match self {
            Self::SingleSh(run) => {
                *run = context.transform(run.to_owned()).await?;
            }

            Self::ComplexSh(complex) => {
                if let Some(wd) = complex.working_dir.as_mut() {
                    *wd = context.transform(wd.to_owned()).await?;
                }
                complex.run = context.transform(complex.run.to_owned()).await?;
            }

            Self::External(external) => {
                if let Some(wd) = external.working_dir.as_mut() {
                    *wd = context.transform(wd.to_owned()).await?;
                }
                external.ext = context.transform(external.ext.to_owned()).await?;
            }
        }
        Ok(())
    }

    pub fn is(&self, name: &str) -> bool {
        if let Self::ComplexSh(complex) = self {
            return complex.name.as_ref().map(|x| x == name).unwrap_or_default();
        }

        if let Self::External(external) = self {
            return external
                .name
                .as_ref()
                .map(|x| x == name)
                .unwrap_or_default();
        }

        return false;
    }
}

#[cfg(feature = "all")]
impl Dependencies for Step {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        match self {
            Self::External(external) if config.full_path(&external.ext).is_yaml() => {
                vec![external.ext.to_owned()]
            }
            Self::SingleSh(_) | Self::ComplexSh { .. } | Self::External { .. } => vec![],
        }
    }
}

impl<'a> Validate<'a> for Step {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            Step::SingleSh(sh) => {
                ctx.validate_symbols(sh);
            }

            Step::ComplexSh(complex) => {
                if let Some(name) = complex.name.as_ref() {
                    ctx.push_section(name);
                }

                if let Some(wd) = complex.working_dir.as_ref() {
                    ctx.push_section("working_dir");
                    ctx.validate_symbols(wd);
                    ctx.pop_section();
                }

                ctx.validate_symbols(&complex.run);

                if complex.name.is_some() {
                    ctx.pop_section();
                }
            }

            Step::External(external) => {
                if let Some(name) = external.name.as_ref() {
                    ctx.push_section(name);
                }

                if let Some(wd) = external.working_dir.as_ref() {
                    ctx.push_section("working_dir");
                    ctx.validate_symbols(wd);
                    ctx.pop_section();
                }

                if ctx.contains_symbols(&external.ext) {
                    ctx.validate_symbols(&external.ext);
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

                if external.name.is_some() {
                    ctx.pop_section();
                }
            }
        }
    }
}
