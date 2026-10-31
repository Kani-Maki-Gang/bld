use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::scalar::{ScalarType, ScalarValue};

#[cfg(feature = "all")]
use {
    crate::{
        expr::v3::{parser, traits::ExprType},
        scalar::single_scalar_type,
        validator::v3::{ExprScope, Validate, ValidatorContext},
    },
    anyhow::{Result, bail},
    std::collections::{BTreeMap, HashMap},
    tracing::debug,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum InputValue {
    Array(Vec<ScalarValue>),
    Scalar(ScalarValue),
}

impl InputValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Scalar(ScalarValue::Text(v)) => Some(v),
            _ => None,
        }
    }
}

impl Display for InputValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn scalar(value: &ScalarValue, quote_text: bool) -> String {
            match value {
                ScalarValue::Number(v) => v.to_string(),
                ScalarValue::Boolean(v) => v.to_string(),
                ScalarValue::Text(v) if quote_text => format!("\"{}\"", v.replace('"', "\\\"")),
                ScalarValue::Text(v) => v.to_string(),
            }
        }

        match self {
            Self::Scalar(value) => f.write_str(&scalar(value, false)),
            Self::Array(items) => {
                let items: Vec<String> = items.iter().map(|x| scalar(x, true)).collect();
                write!(f, "[{}]", items.join(", "))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    #[serde(rename = "type")]
    pub scalar_type: ScalarType,
    pub description: Option<String>,
    pub default: Option<InputValue>,
    #[serde(default)]
    pub required: bool,
}

impl Input {
    pub fn is_required(&self) -> bool {
        self.required
    }

    pub fn default_value(&self) -> Option<&InputValue> {
        self.default.as_ref()
    }
}

#[cfg(test)]
impl Input {
    pub fn typed(scalar_type: ScalarType, default: Option<InputValue>) -> Self {
        Self {
            scalar_type,
            description: None,
            default,
            required: false,
        }
    }

    pub fn text(default: &str) -> Self {
        Self::typed(
            ScalarType::Text,
            Some(InputValue::Scalar(ScalarValue::Text(default.to_string()))),
        )
    }
}

pub fn input_default_text(input: &Input) -> String {
    input
        .default
        .as_ref()
        .map(|x| x.to_string())
        .unwrap_or_default()
}

#[cfg(feature = "all")]
impl Input {
    pub fn expr_type(&self) -> ExprType {
        match (self.scalar_type, &self.default) {
            (ScalarType::Array, Some(InputValue::Array(items))) => {
                ExprType::Array(single_scalar_type(items).map(|x| Box::new(ExprType::from(x))))
            }
            (ty, _) => ExprType::from(ty),
        }
    }
}

/// Orders the inputs that have a default so that every input comes after the inputs
/// that its default reads.
#[cfg(feature = "all")]
pub fn defaults_in_order<'a>(
    inputs: impl IntoIterator<Item = (&'a String, &'a Input)>,
) -> Result<Vec<(&'a str, &'a Input)>> {
    let regex = parser::new_regex()?;
    let mut pending: BTreeMap<&str, (&Input, Vec<String>)> = inputs
        .into_iter()
        .map(|(name, input)| {
            let refs = input
                .default_value()
                .and_then(|x| x.as_text())
                .map(|x| parser::object_refs(&regex, x, "inputs"))
                .unwrap_or_default();
            (name.as_str(), (input, refs))
        })
        .collect();

    let mut order = Vec::with_capacity(pending.len());
    loop {
        let ready: Vec<&str> = pending
            .iter()
            .filter(|(_, (_, refs))| refs.iter().all(|x| !pending.contains_key(x.as_str())))
            .map(|(name, _)| *name)
            .collect();
        if ready.is_empty() {
            break;
        }
        for name in ready {
            if let Some((input, _)) = pending.remove(name) {
                order.push((name, input));
            }
        }
    }

    if !pending.is_empty() {
        let names: Vec<&str> = pending.into_keys().collect();
        bail!(
            "input defaults contain a cyclic reference between: {}",
            names.join(", ")
        );
    }
    Ok(order)
}

#[cfg(feature = "all")]
pub async fn validate_inputs<'a, C: ValidatorContext<'a>>(
    inputs: &'a HashMap<String, Input>,
    ctx: &mut C,
) {
    ctx.push_section("inputs");
    for (name, input) in inputs.iter() {
        debug!("Validating input: {}", name);
        ctx.push_section(name);
        input.validate(ctx).await;
        ctx.pop_section();
    }
    if let Err(e) = defaults_in_order(inputs) {
        ctx.append_error(&e.to_string());
    }
    ctx.pop_section();
}

#[cfg(feature = "all")]
pub fn is_single_expression<'a, C: ValidatorContext<'a>>(ctx: &C, value: &str) -> bool {
    let value = value.trim();
    ctx.expression_count(value) == 1 && value.starts_with("${{") && value.ends_with("}}")
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Input {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        let Some(default) = self.default.as_ref() else {
            return;
        };

        debug!("Validating input default: {default}");
        ctx.push_section("default");
        let expected = self.scalar_type;
        match default {
            InputValue::Array(items) => {
                if expected != ScalarType::Array {
                    ctx.append_error(&format!(
                        "default must be of type {}, found array",
                        expected.as_str()
                    ));
                } else if !items.is_empty() && single_scalar_type(items).is_none() {
                    ctx.append_error("array default must contain values of a single type");
                }
            }

            InputValue::Scalar(ScalarValue::Text(v)) if ctx.contains_expressions(v) => {
                if expected == ScalarType::Text {
                    ctx.validate_expressions(v, ExprScope::StartOfRun);
                } else if !is_single_expression(ctx, v) {
                    ctx.append_error(&format!(
                        "default of a {} input must be a single expression",
                        expected.as_str()
                    ));
                } else {
                    ctx.validate_typed_expression(v, ExprScope::StartOfRun, &self.expr_type());
                }
            }

            InputValue::Scalar(v) => {
                if v.scalar_type() != expected {
                    ctx.append_error(&format!(
                        "default must be of type {}, found {}",
                        expected.as_str(),
                        v.scalar_type().as_str()
                    ));
                }
            }
        }
        ctx.pop_section();
    }
}

#[cfg(test)]
mod tests {
    use super::{Input, InputValue};
    use crate::scalar::{ScalarType, ScalarValue};

    #[test]
    pub fn input_deserializes_without_required() {
        let input: Input = serde_yaml_ng::from_str("type: text\ndefault: ubuntu:22.04").unwrap();

        assert_eq!(input.scalar_type, ScalarType::Text);
        assert!(!input.required);
        assert_eq!(
            input.default,
            Some(InputValue::Scalar(ScalarValue::Text(
                "ubuntu:22.04".to_string()
            )))
        );
    }

    #[test]
    pub fn input_without_type_deserialize_failure() {
        let result: Result<Input, _> = serde_yaml_ng::from_str("default: ubuntu:22.04");
        assert!(result.is_err());
    }

    #[test]
    pub fn input_typed_defaults_deserialize_success() {
        let input: Input = serde_yaml_ng::from_str("type: boolean\ndefault: false").unwrap();
        assert_eq!(
            input.default,
            Some(InputValue::Scalar(ScalarValue::Boolean(false)))
        );

        let input: Input = serde_yaml_ng::from_str("type: number\ndefault: 3").unwrap();
        assert_eq!(
            input.default,
            Some(InputValue::Scalar(ScalarValue::Number(3.0)))
        );

        let input: Input = serde_yaml_ng::from_str("type: array\ndefault: [a, b]").unwrap();
        assert_eq!(
            input.default,
            Some(InputValue::Array(vec![
                ScalarValue::Text("a".to_string()),
                ScalarValue::Text("b".to_string())
            ]))
        );
    }

    #[test]
    pub fn input_value_display_can_be_read_back() {
        let value = InputValue::Array(vec![
            ScalarValue::Text("a \"b\"".to_string()),
            ScalarValue::Text("c".to_string()),
        ]);
        assert_eq!(value.to_string(), r#"["a \"b\"", "c"]"#);
        assert_eq!(
            InputValue::Scalar(ScalarValue::Number(3.5)).to_string(),
            "3.5"
        );
    }
}
