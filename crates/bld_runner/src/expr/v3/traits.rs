#![allow(dead_code)]

use std::{collections::HashMap, fmt::Display, iter::Peekable};

use anyhow::{Result, bail};
use mockall::automock;
use pest::iterators::{Pair, Pairs};

use super::parser::Rule;

fn unescape_string_literal(value: &str) -> String {
    let quote = value.chars().next();
    let inner = if matches!(quote, Some('"') | Some('\'')) && value.len() >= 2 {
        &value[1..value.len() - 1]
    } else {
        value
    };

    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }

        match chars.next() {
            Some('"') => result.push('"'),
            Some('\'') => result.push('\''),
            Some('\\') => result.push('\\'),
            Some('/') => result.push('/'),
            Some('b') => result.push('\u{0008}'),
            Some('f') => result.push('\u{000C}'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Some(decoded) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    result.push(decoded);
                } else {
                    result.push_str("\\u");
                    result.push_str(&hex);
                }
            }
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    result
}

#[derive(Debug, Clone, Eq, Ord, PartialOrd, PartialEq)]
pub enum ExprText<'a> {
    Ref(&'a str),
    Owned(String),
}

impl<'a> ExprText<'a> {
    pub fn inner(&'a self) -> &'a str {
        match self {
            Self::Ref(v) => v,
            Self::Owned(v) => v,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprValue<'a> {
    Boolean(bool),
    /// `raw` keeps the original text the number was parsed from, so that
    /// formatting it back to text does not lose information. `value` is used
    /// for numeric comparisons.
    Number {
        value: f64,
        raw: ExprText<'a>,
    },
    Text(ExprText<'a>),
    Array(Vec<ExprValue<'a>>),
    /// Placeholder used only during validation, when the real value of a
    /// step output is not known yet. It is compatible with every
    /// comparison, so a validation-time expression that compares a step
    /// output does not fail due to a type mismatch.
    Unknown,
}

impl<'a, 'b> ExprValue<'a> {
    pub fn type_as_string(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Number { .. } => "number",
            Self::Text(_) => "text",
            Self::Array(_) => "array",
            Self::Unknown => "unknown",
        }
    }

    /// Checks the value of a condition during validation. The value of an input or of a
    /// step output is not known yet, and both are text, so every text value is accepted
    /// here and only a type that can never be a condition is reported.
    pub fn validate_as_condition(&self) -> Result<()> {
        match self {
            Self::Boolean(_) | Self::Text(_) | Self::Unknown => Ok(()),
            other => bail!(
                "a condition must give a boolean value, but it gives {}",
                other.type_as_string()
            ),
        }
    }

    pub fn try_eq(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Ok(ExprValue::<'b>::Boolean(true));
        }

        let value = match (self, other) {
            (Self::Boolean(l), Self::Boolean(r)) => l == r,
            (Self::Number { value: l, .. }, Self::Number { value: r, .. }) => l == r,
            (Self::Text(l), Self::Text(r)) => l.inner() == r.inner(),
            (Self::Array(l), Self::Array(r)) => {
                if l.len() != r.len() {
                    false
                } else {
                    let mut equal = true;
                    for (l_item, r_item) in l.iter().zip(r.iter()) {
                        let ExprValue::Boolean(item_eq) = l_item.try_eq(r_item)? else {
                            bail!("non boolean type is an invalid comparison result");
                        };
                        if !item_eq {
                            equal = false;
                            break;
                        }
                    }
                    equal
                }
            }
            _ => bail!(
                "cannot compare {} and {}",
                self.type_as_string(),
                other.type_as_string()
            ),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_not_eq(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        let ExprValue::Boolean(value) = self.try_eq(other)? else {
            bail!("non boolean type is an invalid comparison result");
        };
        Ok(ExprValue::<'b>::Boolean(!value))
    }

    pub fn try_ord(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Ok(ExprValue::<'b>::Boolean(true));
        }

        let value = match (self, other) {
            (Self::Number { value: l, .. }, Self::Number { value: r, .. }) => l > r,
            (Self::Text(l), Self::Text(r)) => l.inner() > r.inner(),
            (Self::Boolean(l), Self::Boolean(r)) => l > r,
            _ => bail!(
                "cannot compare {} and {}",
                self.type_as_string(),
                other.type_as_string()
            ),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_and(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Ok(ExprValue::<'b>::Boolean(true));
        }

        let value = match (self, other) {
            (Self::Boolean(l), Self::Boolean(r)) => *l && *r,
            _ => bail!(
                "cannot use logical AND comparison on type {} and {}",
                self.type_as_string(),
                other.type_as_string()
            ),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_or(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Ok(ExprValue::<'b>::Boolean(true));
        }

        let value = match (self, other) {
            (Self::Boolean(l), Self::Boolean(r)) => *l || *r,
            _ => bail!(
                "cannot use logical OR comparison on type {} and {}",
                self.type_as_string(),
                other.type_as_string()
            ),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }
}

impl<'b> TryFrom<&'b str> for ExprValue<'_> {
    type Error = anyhow::Error;

    fn try_from(value: &'b str) -> Result<Self> {
        // Try number
        if let Ok(num) = value.parse::<f64>()
            && num.is_finite()
        {
            return Ok(ExprValue::Number {
                value: num,
                raw: ExprText::Owned(value.to_string()),
            });
        }

        // Try boolean
        if let Ok(boolean) = value.parse::<bool>() {
            return Ok(ExprValue::Boolean(boolean));
        }

        // Try array
        if value.starts_with('[') && value.ends_with(']') {
            let mut expr_type: Option<&'static str> = None;
            let mut expr_value = vec![];
            for entry in value[1..value.len() - 1].split(',') {
                let entry_expr_value: ExprValue<'_> = entry.trim().try_into()?;
                let entry_expr_type = entry_expr_value.type_as_string();

                if let Some(expr_type) = expr_type
                    && expr_type != entry_expr_value.type_as_string()
                {
                    bail!("Array expression contains entries of multiple types")
                }

                expr_type = Some(entry_expr_type);
                expr_value.push(entry_expr_value);
            }
            return Ok(ExprValue::Array(expr_value));
        }

        // Fallback to test
        Ok(ExprValue::Text(ExprText::Owned(unescape_string_literal(
            value,
        ))))
    }
}

impl TryInto<bool> for ExprValue<'_> {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<bool> {
        match self {
            Self::Boolean(value) => Ok(value),
            Self::Text(text) if text.inner() == "true" => Ok(true),
            Self::Text(text) if text.inner() == "false" => Ok(false),
            other => bail!(
                "a condition must give a boolean value, but it gives {}",
                other.type_as_string()
            ),
        }
    }
}

impl TryFrom<String> for ExprValue<'_> {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        value.as_str().try_into()
    }
}

impl Display for ExprValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Boolean(value) => value.to_string(),
            Self::Number { raw, .. } => raw.inner().to_string(),
            Self::Text(ExprText::Ref(value)) => value.to_string(),
            Self::Text(ExprText::Owned(value)) => value.to_string(),
            Self::Array(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Unknown => "unknown".to_string(),
        };
        f.write_str(&value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputScope {
    Step,
    Job,
}

pub trait ReadonlyRuntimeExprContext<'a> {
    fn get_root_dir(&'a self) -> &'a str;
    fn get_project_dir(&'a self) -> &'a str;
    fn get_input(&'a self, name: &'a str) -> Result<&'a str>;
    fn get_env(&'a self, name: &'a str) -> Result<&'a str>;
    fn get_run_id(&'a self) -> &'a str;
    fn get_run_start_time(&'a self) -> &'a str;
}

#[automock]
pub trait WritableRuntimeExprContext {
    #[allow(clippy::needless_lifetimes)]
    fn get_exec_id<'a>(&'a self) -> Option<&'a str>;
    fn get_output<'a>(&'a self, scope: OutputScope, id: &str, name: &str) -> Result<ExprValue<'a>>;
    fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()>;
    fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()>;
    #[allow(clippy::needless_lifetimes)]
    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<&'a str>;
}

pub trait EvalObject<'a> {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>>;
}

pub trait EvalExpr<'a> {
    fn eval_cmp(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>>;
    fn eval_symbol(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>>;
    fn eval_expr(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>>;
    fn eval_logical_expr(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>>;
    fn eval(&self, expr: &'a str) -> Result<ExprValue<'a>>;
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use anyhow::Result;

    pub fn expr_number<'a>(value: f64) -> ExprValue<'a> {
        ExprValue::Number {
            value,
            raw: ExprText::Owned(value.to_string()),
        }
    }

    #[test]
    fn number_conversion_keeps_leading_and_trailing_zeros() {
        let value: ExprValue = "007".try_into().unwrap();
        assert_eq!(value.to_string(), "007");
        let ExprValue::Number { value: num, .. } = value else {
            panic!("expected number");
        };
        assert_eq!(num, 7.0);

        let value: ExprValue = "1.10".try_into().unwrap();
        assert_eq!(value.to_string(), "1.10");
        let ExprValue::Number { value: num, .. } = value else {
            panic!("expected number");
        };
        assert_eq!(num, 1.1);
    }

    #[test]
    fn number_conversion_rejects_non_finite_values() {
        for input in ["NaN", "inf", "-inf", "infinity"] {
            let value: ExprValue = input.try_into().unwrap();
            assert!(
                matches!(value, ExprValue::Text(_)),
                "expected {input} to stay text, got {value:?}"
            );
            assert_eq!(value.to_string(), input);
        }
    }

    #[test]
    fn numbers_still_compare_by_value() {
        let count: ExprValue = "10".try_into().unwrap();
        let threshold = expr_number(3.0);
        let ExprValue::Boolean(greater) = count.try_ord(&threshold).unwrap() else {
            panic!("expected boolean");
        };
        assert!(greater);

    #[test]
    fn boolean_true_and_false_are_used_as_is() {
        assert!(TryInto::<bool>::try_into(ExprValue::Boolean(true)).unwrap());
        assert!(!TryInto::<bool>::try_into(ExprValue::Boolean(false)).unwrap());
    }

    #[test]
    fn text_true_and_false_are_accepted_since_inputs_are_always_text() {
        assert!(
            TryInto::<bool>::try_into(ExprValue::Text(ExprText::Owned("true".to_string())))
                .unwrap()
        );
        assert!(
            !TryInto::<bool>::try_into(ExprValue::Text(ExprText::Owned("false".to_string())))
                .unwrap()
        );
    }

    #[test]
    fn number_gives_an_error_naming_the_type() {
        let res: Result<bool> = ExprValue::Number(1.0).try_into();
        assert!(res.unwrap_err().to_string().contains("number"));
    }

    #[test]
    fn array_gives_an_error() {
        assert!(TryInto::<bool>::try_into(ExprValue::Array(vec![])).is_err());
    }

    #[test]
    fn arbitrary_text_gives_an_error() {
        let res: Result<bool> = ExprValue::Text(ExprText::Owned("yes".to_string())).try_into();
        assert!(res.unwrap_err().to_string().contains("text"));
    }

    #[test]
    fn validation_accepts_every_text_and_an_unknown_value() {
        assert!(
            ExprValue::Text(ExprText::Owned(String::new()))
                .validate_as_condition()
                .is_ok()
        );
        assert!(
            ExprValue::Text(ExprText::Owned("yes".to_string()))
                .validate_as_condition()
                .is_ok()
        );
        assert!(ExprValue::Unknown.validate_as_condition().is_ok());
        assert!(ExprValue::Boolean(false).validate_as_condition().is_ok());
    }

    #[test]
    fn validation_rejects_a_number_and_an_array() {
        let err = ExprValue::Number(1.0).validate_as_condition().unwrap_err();
        assert!(err.to_string().contains("number"));
        assert!(ExprValue::Array(vec![]).validate_as_condition().is_err());
    }
}
