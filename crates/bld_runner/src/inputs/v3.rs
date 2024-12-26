use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;

#[cfg(feature = "all")]
use crate::validator::v3::{Validate, ValidatorContext};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
    Simple(String),
    Complex {
        description: Option<String>,
        default: Option<String>,
        required: bool,
    },
}

impl Input {
    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &'a ExecutionContext<'a>) -> Result<()> {
        match self {
            Input::Simple(v) => {
                *v = context.transform(v.to_owned()).await?;
            }
            Input::Complex { default, .. } => {
                if let Some(v) = default {
                    *default = Some(context.transform(v.to_owned()).await?);
                }
            }
        }
        Ok(())
    }

    pub fn is_required(&self) -> bool {
        match self {
            Input::Simple(_) => false,
            Input::Complex { required, .. } => *required,
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Input {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            Input::Simple(v) => {
                debug!("Validating input: {}", v);
                ctx.validate_symbols(v);
            }
            Input::Complex { default, .. } => {
                if let Some(v) = default {
                    ctx.push_section("default");
                    ctx.validate_symbols(v);
                    ctx.pop_section();
                }
            }
        }
    }
}
