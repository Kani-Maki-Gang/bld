use crate::external::v3::External;
use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;

#[cfg(feature = "all")]
use crate::validator::v3::{Validate, ValidatorContext};

#[cfg(feature = "all")]
use crate::traits::Dependencies;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    pub name: Option<String>,
    pub working_dir: Option<String>,
    pub run: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
    SingleSh(String),
    ComplexSh(Box<ShellCommand>),
    ExternalFile(Box<External>),
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

            Self::ExternalFile(external) => {
                external.apply_tokens(context).await?;
            }
        }
        Ok(())
    }

    pub fn is(&self, name: &str) -> bool {
        if let Self::ComplexSh(complex) = self {
            return complex.name.as_ref().map(|x| x == name).unwrap_or_default();
        }

        if let Self::ExternalFile(external) = self {
            return external
                .name
                .as_ref()
                .map(|x| x == name)
                .unwrap_or_default();
        }

        false
    }
}

#[cfg(feature = "all")]
impl Dependencies for Step {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        match self {
            Self::ExternalFile(external) if config.full_path(&external.uses).is_yaml() => {
                vec![external.uses.to_owned()]
            }
            Self::SingleSh(_) | Self::ComplexSh { .. } | Self::ExternalFile { .. } => vec![],
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Step {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            Step::SingleSh(sh) => {
                debug!("Step is a single shell command");
                ctx.validate_symbols(sh);
            }

            Step::ComplexSh(complex) => {
                debug!("Step is a complex shell command");
                if let Some(name) = complex.name.as_ref() {
                    ctx.push_section(name);
                }

                if let Some(wd) = complex.working_dir.as_ref() {
                    debug!("Validating step's working directory");
                    ctx.push_section("working_dir");
                    ctx.validate_symbols(wd);
                    ctx.pop_section();
                }

                debug!("Validating step's run command");
                ctx.validate_symbols(&complex.run);

                if complex.name.is_some() {
                    ctx.pop_section();
                }
            }

            Step::ExternalFile(external) => {
                debug!("Step is an external file");
                external.validate(ctx).await;
            }
        }
    }
}
