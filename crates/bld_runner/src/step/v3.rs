use std::iter::Peekable;

use crate::{
    expr::v3::{
        parser::Rule,
        traits::{
            EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
        },
    },
    external::v3::External,
};
use anyhow::{Result, bail};
use pest::iterators::Pairs;
use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
use crate::{
    traits::Dependencies,
    validator::v3::{Validate, ValidatorContext},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    pub id: Option<String>,
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
    pub fn is(&self, id: &str) -> bool {
        let Self::ComplexSh(complex) = self else {
            return false;
        };
        complex.id.as_ref().map(|x| x == id).unwrap_or_default()
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
impl<'a> EvalObject<'a> for Step {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        _rctx: &'a RCtx,
        _wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };
        let value = match self {
            Self::SingleSh(_) => {
                bail!("invalid expression for step");
            }

            Self::ExternalFile(_) => {
                // TODO: Remove once external section is removed.
                bail!("invalid expression for step");
            }

            Self::ComplexSh(command) => match object.as_span().as_str() {
                "name" => command.name.as_ref().map(|x| x.as_str()).unwrap_or(""),
                "working_dir" => command
                    .working_dir
                    .as_ref()
                    .map(|x| x.as_str())
                    .unwrap_or(""),
                "run" => &command.run,
                value => bail!("invalid steps field: {value}"),
            },
        };
        Ok(ExprValue::Text(ExprText::Ref(value)))
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
                if let Some(id) = complex.id.as_ref() {
                    ctx.push_section(id);
                }

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

#[cfg(test)]
mod tests {}
