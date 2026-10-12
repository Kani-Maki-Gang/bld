#![allow(dead_code)]

use super::parser::{ExprParser, Rule};
use anyhow::{Result, bail};
use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use std::{collections::HashMap, fmt::Display, iter::Peekable};

use crate::scalar::ScalarValue;

#[cfg(test)]
use mockall::automock;

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

pub fn array_from_pair<'a>(pair: Pair<'_, Rule>) -> Result<ExprValue<'a>> {
    let Rule::Array = pair.as_rule() else {
        bail!("expected array rule, found {:?}", pair.as_rule());
    };

    let mut items = Vec::new();
    let mut element_type: Option<&'static str> = None;

    for element in pair.into_inner() {
        let Rule::ArrayElement = element.as_rule() else {
            bail!("expected array element rule, found {:?}", element.as_rule());
        };

        let value: ExprValue<'a> = element.as_span().as_str().try_into()?;

        match element_type {
            Some(expected) if expected != value.type_as_string() => {
                bail!("array elements must all be of the same type")
            }
            None => element_type = Some(value.type_as_string()),
            _ => {}
        }

        items.push(value);
    }

    Ok(ExprValue::Array(items))
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprValue<'a> {
    Boolean(bool),
    Number {
        value: f64,
        raw: ExprText<'a>, // original text for formatting puprposes
    },
    Text(ExprText<'a>),
    Array(Vec<ExprValue<'a>>),
    Unknown, // Placeholder for expression results that aren't known at validation time
}

/// Converts `text` into the same variant as `target`, so that a value coming from an
/// always-text source (inputs, env, a quoted literal) can be compared against a typed one
/// (a step output, a matrix value, a literal number/boolean/array) instead of failing outright.
fn coerce_text_like<'c>(text: &str, target: &ExprValue<'c>) -> Option<ExprValue<'c>> {
    match target {
        ExprValue::Number { .. } => {
            text.parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map(|value| ExprValue::Number {
                    value,
                    raw: ExprText::Owned(text.to_string()),
                })
        }
        ExprValue::Boolean(_) => text.parse::<bool>().ok().map(ExprValue::Boolean),
        ExprValue::Array(_) => match ExprValue::try_from(text) {
            Ok(value @ ExprValue::Array(_)) => Some(value),
            _ => None,
        },
        ExprValue::Text(_) | ExprValue::Unknown => None,
    }
}

/// Coerces either operand that is text into the other operand's type, when possible, so that
/// comparisons behave the same no matter which source the value came from.
fn coerce_for_cmp<'c>(l: &ExprValue<'c>, r: &ExprValue<'c>) -> (ExprValue<'c>, ExprValue<'c>) {
    match (l, r) {
        (ExprValue::Text(t), _) => match coerce_text_like(t.inner(), r) {
            Some(coerced) => (coerced, r.clone()),
            None => (l.clone(), r.clone()),
        },
        (_, ExprValue::Text(t)) => match coerce_text_like(t.inner(), l) {
            Some(coerced) => (l.clone(), coerced),
            None => (l.clone(), r.clone()),
        },
        _ => (l.clone(), r.clone()),
    }
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

    /// Reads a logical value out of a boolean, or out of the text `true`/`false`, the same way
    /// a condition already does.
    pub fn as_logical(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::Text(text) if text.inner() == "true" => Some(true),
            Self::Text(text) if text.inner() == "false" => Some(false),
            _ => None,
        }
    }

    pub fn try_eq(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Ok(ExprValue::<'b>::Boolean(true));
        }

        let (l, r) = coerce_for_cmp(self, other);
        let value = match (&l, &r) {
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

        let (l, r) = coerce_for_cmp(self, other);
        let value = match (&l, &r) {
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

        match (self.as_logical(), other.as_logical()) {
            (Some(l), Some(r)) => Ok(ExprValue::<'b>::Boolean(l && r)),
            _ => bail!(
                "cannot use logical AND comparison on type {} and {}",
                self.type_as_string(),
                other.type_as_string()
            ),
        }
    }

    pub fn try_or(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Ok(ExprValue::<'b>::Boolean(true));
        }

        match (self.as_logical(), other.as_logical()) {
            (Some(l), Some(r)) => Ok(ExprValue::<'b>::Boolean(l || r)),
            _ => bail!(
                "cannot use logical OR comparison on type {} and {}",
                self.type_as_string(),
                other.type_as_string()
            ),
        }
    }

    pub fn try_negate(&self) -> Result<ExprValue<'b>> {
        match self {
            ExprValue::Unknown => Ok(ExprValue::Unknown),
            other => match other.as_logical() {
                Some(value) => Ok(ExprValue::Boolean(!value)),
                None => bail!("cannot use logical NOT on type {}", other.type_as_string()),
            },
        }
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
        if value.starts_with('[')
            && value.ends_with(']')
            && let Ok(mut pairs) = ExprParser::parse(Rule::Array, value)
            && let Some(pair) = pairs.next()
            && pair.as_span().end() == value.len()
        {
            return array_from_pair(pair);
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

impl From<&ScalarValue> for ExprValue<'_> {
    fn from(value: &ScalarValue) -> Self {
        match value {
            ScalarValue::Text(t) => Self::Text(ExprText::Owned(t.to_owned())),
            ScalarValue::Number(v) => Self::Number {
                value: *v,
                raw: ExprText::Owned(v.to_string()),
            },
            ScalarValue::Boolean(b) => Self::Boolean(*b),
        }
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
    fn get_input(&'a self, name: &'a str) -> Result<ExprValue<'a>>;
    fn get_env(&'a self, name: &'a str) -> Result<ExprValue<'a>>;
    fn get_run_id(&'a self) -> &'a str;
    fn get_run_start_time(&'a self) -> &'a str;
}

#[cfg_attr(test, automock)]
pub trait WritableRuntimeExprContext {
    #[allow(clippy::needless_lifetimes)]
    fn get_exec_id<'a>(&'a self) -> Option<&'a str>;
    fn get_output<'a>(&'a self, scope: OutputScope, id: &str, name: &str) -> Result<ExprValue<'a>>;
    fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()>;
    fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()>;
    #[allow(clippy::needless_lifetimes)]
    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<ExprValue<'a>>;
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
    fn scalar_number_into_expr_value_success() {
        let value: ExprValue = (&ScalarValue::Number(1.0)).into();
        assert_eq!(value.to_string(), "1");
        let ExprValue::Number { value: num, .. } = value else {
            panic!("expected number");
        };
        assert_eq!(num, 1.0);
    }

    #[test]
    fn scalar_boolean_into_expr_value_success() {
        let value: ExprValue = (&ScalarValue::Boolean(true)).into();
        assert_eq!(value, ExprValue::Boolean(true));
    }

    #[test]
    fn scalar_text_into_expr_value_success() {
        let value: ExprValue = (&ScalarValue::Text("a".to_string())).into();
        let ExprValue::Text(text) = value else {
            panic!("expected text");
        };
        assert_eq!(text.inner(), "a");
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

    fn array_items(value: &str) -> Vec<String> {
        let value: ExprValue = value.try_into().unwrap();
        let ExprValue::Array(items) = value else {
            panic!("expected array, got {value:?}");
        };
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn array_conversion_keeps_a_comma_inside_quotation_marks() {
        assert_eq!(array_items(r#"["a,b", "c"]"#), vec!["a,b", "c"]);
    }

    #[test]
    fn array_conversion_accepts_an_empty_array() {
        assert!(array_items("[]").is_empty());
    }

    #[test]
    fn array_conversion_accepts_text_and_number_elements() {
        assert_eq!(array_items(r#"["x", "y"]"#), vec!["x", "y"]);
        assert_eq!(array_items("[1, 2]"), vec!["1", "2"]);
    }

    #[test]
    fn array_conversion_rejects_elements_without_quotation_marks() {
        let value: ExprValue = "[a, b]".try_into().unwrap();
        assert!(
            matches!(value, ExprValue::Text(_)),
            "expected text, got {value:?}"
        );
        assert_eq!(value.to_string(), "[a, b]");
    }

    #[test]
    fn array_conversion_rejects_text_after_the_array() {
        let value: ExprValue = r#"["a"] junk]"#.try_into().unwrap();
        assert!(
            matches!(value, ExprValue::Text(_)),
            "expected text, got {value:?}"
        );
    }

    #[test]
    fn array_conversion_rejects_elements_of_multiple_types() {
        let error = TryInto::<ExprValue>::try_into(r#"[1, "a"]"#).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("array elements must all be of the same type"),
            "error was: {error}"
        );
    }

    #[test]
    fn numbers_still_compare_by_value() {
        let count: ExprValue = "10".try_into().unwrap();
        let threshold = expr_number(3.0);
        let ExprValue::Boolean(greater) = count.try_ord(&threshold).unwrap() else {
            panic!("expected boolean");
        };
        assert!(greater);
    }

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
        let res: Result<bool> = expr_number(1.0).try_into();
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
    fn try_negate_success() {
        assert_eq!(
            ExprValue::Boolean(true).try_negate().unwrap(),
            ExprValue::Boolean(false)
        );
        assert_eq!(
            ExprValue::Boolean(false).try_negate().unwrap(),
            ExprValue::Boolean(true)
        );
        assert_eq!(ExprValue::Unknown.try_negate().unwrap(), ExprValue::Unknown);
    }

    #[test]
    fn try_negate_accepts_text_true_and_false() {
        let text = ExprValue::Text(ExprText::Owned("true".to_string()));
        assert_eq!(text.try_negate().unwrap(), ExprValue::Boolean(false));
        let text = ExprValue::Text(ExprText::Owned("false".to_string()));
        assert_eq!(text.try_negate().unwrap(), ExprValue::Boolean(true));
    }

    #[test]
    fn try_negate_failure() {
        let text = ExprValue::Text(ExprText::Owned("yes".to_string()));
        assert!(text.try_negate().unwrap_err().to_string().contains("text"));
        assert!(
            expr_number(1.0)
                .try_negate()
                .unwrap_err()
                .to_string()
                .contains("number")
        );
        assert!(
            ExprValue::Array(vec![])
                .try_negate()
                .unwrap_err()
                .to_string()
                .contains("array")
        );
    }

    #[test]
    fn try_eq_coerces_text_to_boolean() {
        let text = ExprValue::Text(ExprText::Owned("true".to_string()));
        assert_eq!(
            text.try_eq(&ExprValue::Boolean(true)).unwrap(),
            ExprValue::Boolean(true)
        );
        assert_eq!(
            ExprValue::Boolean(true).try_eq(&text).unwrap(),
            ExprValue::Boolean(true)
        );
    }

    #[test]
    fn try_eq_coerces_text_to_number() {
        let text = ExprValue::Text(ExprText::Owned("3".to_string()));
        assert_eq!(
            text.try_eq(&expr_number(3.0)).unwrap(),
            ExprValue::Boolean(true)
        );
    }

    #[test]
    fn try_eq_leaves_non_convertible_text_as_is() {
        let text = ExprValue::Text(ExprText::Owned("abc".to_string()));
        let error = text.try_eq(&expr_number(3.0)).unwrap_err().to_string();
        assert!(
            error.contains("text") && error.contains("number"),
            "{error}"
        );
    }

    #[test]
    fn try_ord_coerces_text_to_number() {
        let count = ExprValue::Text(ExprText::Owned("10".to_string()));
        assert_eq!(
            count.try_ord(&expr_number(3.0)).unwrap(),
            ExprValue::Boolean(true)
        );
    }

    #[test]
    fn try_and_accepts_text_true_and_false() {
        let deploy = ExprValue::Text(ExprText::Owned("false".to_string()));
        assert_eq!(
            deploy.try_and(&ExprValue::Boolean(true)).unwrap(),
            ExprValue::Boolean(false)
        );
        let deploy = ExprValue::Text(ExprText::Owned("true".to_string()));
        assert_eq!(
            deploy.try_and(&ExprValue::Boolean(true)).unwrap(),
            ExprValue::Boolean(true)
        );
    }

    #[test]
    fn try_or_accepts_text_true_and_false() {
        let deploy = ExprValue::Text(ExprText::Owned("false".to_string()));
        assert_eq!(
            deploy.try_or(&ExprValue::Boolean(false)).unwrap(),
            ExprValue::Boolean(false)
        );
        let deploy = ExprValue::Text(ExprText::Owned("true".to_string()));
        assert_eq!(
            deploy.try_or(&ExprValue::Boolean(false)).unwrap(),
            ExprValue::Boolean(true)
        );
    }

    #[test]
    fn try_eq_coerces_text_to_array() {
        let oses = ExprValue::Text(ExprText::Owned(r#"["a","b"]"#.to_string()));
        let literal: ExprValue = r#"["a","b"]"#.try_into().unwrap();
        assert_eq!(oses.try_eq(&literal).unwrap(), ExprValue::Boolean(true));
    }
}
