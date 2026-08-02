use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use {
    crate::validator::v3::{ExprScope, Validate, ValidatorContext},
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
        #[serde(default)]
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

    pub fn default_value(&self) -> Option<&str> {
        match self {
            Input::Simple(v) => Some(v),
            Input::Complex { default, .. } => default.as_deref(),
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
                ctx.validate_expressions(v, ExprScope::StartOfRun);
            }
            Input::Complex { default, .. } => {
                if let Some(v) = default {
                    ctx.push_section("default");
                    ctx.validate_expressions(v, ExprScope::StartOfRun);
                    ctx.pop_section();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Input;

    #[test]
    pub fn complex_input_deserializes_without_required() {
        let input: Input = serde_yaml_ng::from_str("default: ubuntu:22.04").unwrap();

        assert!(matches!(
            input,
            Input::Complex {
                default: Some(default),
                required: false,
                ..
            } if default == "ubuntu:22.04"
        ));
    }
}
