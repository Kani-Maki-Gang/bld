use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use {
    crate::validator::v3::{Validate, ValidatorContext},
    anyhow::{Error, Result, anyhow},
    tracing::debug,
};

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
    pub fn is_required(&self) -> bool {
        match self {
            Input::Simple(_) => false,
            Input::Complex { required, .. } => *required,
        }
    }
}

#[cfg(feature = "all")]
impl<'a> TryInto<&'a str> for &'a Input {
    type Error = Error;

    fn try_into(self) -> Result<&'a str, Self::Error> {
        match self {
            Input::Simple(v) => Ok(v),
            Input::Complex { default, .. } => default
                .as_deref()
                .ok_or_else(|| anyhow!("default value not found")),
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
