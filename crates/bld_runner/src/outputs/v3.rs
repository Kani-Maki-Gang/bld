use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use {
    crate::validator::v3::{ExprScope, Validate, ValidatorContext},
    tracing::debug,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Output {
    Simple(String),
    Complex {
        description: Option<String>,
        value: String,
    },
}

impl Output {
    pub fn value(&self) -> &str {
        match self {
            Output::Simple(v) => v,
            Output::Complex { value, .. } => value,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Output::Simple(_) => None,
            Output::Complex { description, .. } => description.as_deref(),
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Output {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            Output::Simple(v) => {
                debug!("Validating output: {}", v);
                ctx.validate_expressions(v, ExprScope::Runtime);
            }
            Output::Complex { value, .. } => {
                debug!("Validating output: {}", value);
                ctx.push_section("value");
                ctx.validate_expressions(value, ExprScope::Runtime);
                ctx.pop_section();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Output;

    #[test]
    pub fn simple_output_deserializes_from_a_string() {
        let output: Output = serde_yaml_ng::from_str("${{ steps.build.outputs.digest }}").unwrap();

        assert!(matches!(
            output,
            Output::Simple(value) if value == "${{ steps.build.outputs.digest }}"
        ));
    }

    #[test]
    pub fn complex_output_deserializes_with_description() {
        let output: Output = serde_yaml_ng::from_str(
            "description: The tag of the image\nvalue: ${{ steps.push.outputs.tag }}",
        )
        .unwrap();

        assert!(matches!(
            output,
            Output::Complex {
                description: Some(description),
                value,
            } if description == "The tag of the image" && value == "${{ steps.push.outputs.tag }}"
        ));
    }

    #[test]
    pub fn complex_output_deserializes_without_description() {
        let output: Output =
            serde_yaml_ng::from_str("value: ${{ steps.push.outputs.tag }}").unwrap();

        assert!(matches!(
            output,
            Output::Complex {
                description: None,
                value,
            } if value == "${{ steps.push.outputs.tag }}"
        ));
    }
}
