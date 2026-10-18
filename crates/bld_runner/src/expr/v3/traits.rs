#![allow(dead_code)]

use super::parser::{ExprParser, Rule};
use anyhow::{Result, anyhow, bail};
use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use std::{collections::HashMap, fmt::Display, iter::Peekable};

use crate::scalar::{ScalarType, ScalarValue};

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
pub enum ExprType {
    Boolean,
    Number,
    Text,
    Array(Option<Box<ExprType>>), // the type of the elements, when known
}

impl ExprType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Text => "text",
            Self::Array(_) => "array",
        }
    }

    pub fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Array(Some(l)), Self::Array(Some(r))) => l.matches(r),
            (Self::Array(_), Self::Array(_)) => true,
            (l, r) => l == r,
        }
    }
}

impl Display for ExprType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<ScalarType> for ExprType {
    fn from(value: ScalarType) -> Self {
        match value {
            ScalarType::Boolean => Self::Boolean,
            ScalarType::Number => Self::Number,
            ScalarType::Text => Self::Text,
            ScalarType::Array => Self::Array(None),
        }
    }
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
    Unknown(Option<ExprType>), // Placeholder for expression results that aren't known at validation time
}

impl<'a, 'b> ExprValue<'a> {
    pub fn type_as_string(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Number { .. } => "number",
            Self::Text(_) => "text",
            Self::Array(_) => "array",
            Self::Unknown(Some(ty)) => ty.as_str(),
            Self::Unknown(None) => "unknown",
        }
    }

    pub fn expr_type(&self) -> Option<ExprType> {
        match self {
            Self::Boolean(_) => Some(ExprType::Boolean),
            Self::Number { .. } => Some(ExprType::Number),
            Self::Text(_) => Some(ExprType::Text),
            Self::Array(items) => Some(ExprType::Array(
                items.first().and_then(|x| x.expr_type()).map(Box::new),
            )),
            Self::Unknown(ty) => ty.clone(),
        }
    }

    pub fn into_owned(self) -> ExprValue<'static> {
        match self {
            Self::Boolean(value) => ExprValue::Boolean(value),
            Self::Number { value, raw } => ExprValue::Number {
                value,
                raw: ExprText::Owned(raw.inner().to_string()),
            },
            Self::Text(text) => ExprValue::Text(ExprText::Owned(text.inner().to_string())),
            Self::Array(items) => {
                ExprValue::Array(items.into_iter().map(|x| x.into_owned()).collect())
            }
            Self::Unknown(ty) => ExprValue::Unknown(ty),
        }
    }

    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown(_))
    }

    fn try_unknown_operation(
        &self,
        other: &Self,
        allowed: impl Fn(&ExprType, &ExprType) -> bool,
        error: impl FnOnce() -> anyhow::Error,
    ) -> Result<ExprValue<'b>> {
        match (self.expr_type(), other.expr_type()) {
            (Some(l), Some(r)) if !allowed(&l, &r) => Err(error()),
            _ => Ok(ExprValue::<'b>::Unknown(Some(ExprType::Boolean))),
        }
    }

    pub fn try_eq(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        let cmp_error = || {
            anyhow!(
                "cannot compare {} and {}",
                self.type_as_string(),
                other.type_as_string()
            )
        };

        if self.is_unknown() || other.is_unknown() {
            return self.try_unknown_operation(other, |l, r| l.matches(r), cmp_error);
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
            _ => return Err(cmp_error()),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_not_eq(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        match self.try_eq(other)? {
            ExprValue::Boolean(value) => Ok(ExprValue::<'b>::Boolean(!value)),
            value @ ExprValue::Unknown(_) => Ok(value),
            _ => bail!("non boolean type is an invalid comparison result"),
        }
    }

    pub fn try_ord(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        let cmp_error = || {
            anyhow!(
                "cannot compare {} and {}",
                self.type_as_string(),
                other.type_as_string()
            )
        };

        if self.is_unknown() || other.is_unknown() {
            return self.try_unknown_operation(
                other,
                |l, r| l == r && !matches!(l, ExprType::Array(_)),
                cmp_error,
            );
        }

        let value = match (self, other) {
            (Self::Number { value: l, .. }, Self::Number { value: r, .. }) => l > r,
            (Self::Text(l), Self::Text(r)) => l.inner() > r.inner(),
            (Self::Boolean(l), Self::Boolean(r)) => l > r,
            _ => return Err(cmp_error()),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_and(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        let and_error = || {
            anyhow!(
                "cannot use logical AND comparison on type {} and {}",
                self.type_as_string(),
                other.type_as_string()
            )
        };

        if self.is_unknown() || other.is_unknown() {
            return self.try_unknown_operation(other, both_boolean, and_error);
        }

        let value = match (self, other) {
            (Self::Boolean(l), Self::Boolean(r)) => *l && *r,
            _ => return Err(and_error()),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_or(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        let or_error = || {
            anyhow!(
                "cannot use logical OR comparison on type {} and {}",
                self.type_as_string(),
                other.type_as_string()
            )
        };

        if self.is_unknown() || other.is_unknown() {
            return self.try_unknown_operation(other, both_boolean, or_error);
        }

        let value = match (self, other) {
            (Self::Boolean(l), Self::Boolean(r)) => *l || *r,
            _ => return Err(or_error()),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_negate(&self) -> Result<ExprValue<'b>> {
        let value = match self {
            ExprValue::Boolean(val) => ExprValue::Boolean(!val),
            ExprValue::Unknown(None | Some(ExprType::Boolean)) => {
                ExprValue::Unknown(Some(ExprType::Boolean))
            }
            val => bail!("cannot use logical NOT on type {}", val.type_as_string()),
        };
        Ok(value)
    }
}

fn both_boolean(l: &ExprType, r: &ExprType) -> bool {
    matches!((l, r), (ExprType::Boolean, ExprType::Boolean))
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
            Self::Unknown(_) => "unknown".to_string(),
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
        assert_eq!(
            ExprValue::Unknown(None).try_negate().unwrap(),
            ExprValue::Unknown(Some(ExprType::Boolean))
        );
    }

    #[test]
    fn try_negate_failure() {
        let text = ExprValue::Text(ExprText::Owned("true".to_string()));
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

    fn typed(ty: ExprType) -> ExprValue<'static> {
        ExprValue::Unknown(Some(ty))
    }

    #[test]
    fn typed_unknown_comparison_of_the_same_type_success() {
        let boolean = typed(ExprType::Boolean);
        let number = typed(ExprType::Number);
        let unknown_bool = ExprValue::Unknown(Some(ExprType::Boolean));

        assert_eq!(
            boolean.try_eq(&ExprValue::Boolean(true)).unwrap(),
            unknown_bool
        );
        assert_eq!(
            boolean.try_not_eq(&ExprValue::Boolean(true)).unwrap(),
            unknown_bool
        );
        assert_eq!(number.try_ord(&expr_number(3.0)).unwrap(), unknown_bool);
        assert_eq!(
            boolean.try_and(&ExprValue::Boolean(true)).unwrap(),
            unknown_bool
        );
        assert_eq!(
            ExprValue::Boolean(false).try_or(&boolean).unwrap(),
            unknown_bool
        );
        assert_eq!(boolean.try_negate().unwrap(), unknown_bool);
    }

    #[test]
    fn typed_unknown_comparison_of_different_types_failure() {
        let text = typed(ExprType::Text);
        let error = text
            .try_eq(&ExprValue::Boolean(true))
            .unwrap_err()
            .to_string();
        assert_eq!(error, "cannot compare text and boolean");

        let error = text.try_ord(&expr_number(3.0)).unwrap_err().to_string();
        assert_eq!(error, "cannot compare text and number");

        let error = text
            .try_and(&ExprValue::Boolean(true))
            .unwrap_err()
            .to_string();
        assert!(error.contains("logical AND"), "{error}");

        let error = typed(ExprType::Number)
            .try_or(&typed(ExprType::Boolean))
            .unwrap_err()
            .to_string();
        assert!(error.contains("logical OR"), "{error}");

        let error = text.try_negate().unwrap_err().to_string();
        assert!(error.contains("logical NOT on type text"), "{error}");

        let array = typed(ExprType::Array(None));
        assert!(array.try_ord(&array.clone()).is_err());
    }

    #[test]
    fn unknown_output_comparison_with_any_type_success() {
        let output = ExprValue::Unknown(None);
        let unknown_bool = ExprValue::Unknown(Some(ExprType::Boolean));
        assert_eq!(
            output.try_eq(&ExprValue::Boolean(true)).unwrap(),
            unknown_bool
        );
        assert_eq!(output.try_eq(&typed(ExprType::Text)).unwrap(), unknown_bool);
        assert_eq!(output.try_ord(&expr_number(3.0)).unwrap(), unknown_bool);
        assert_eq!(
            output.try_and(&ExprValue::Boolean(true)).unwrap(),
            unknown_bool
        );
    }

    #[test]
    fn typed_unknown_array_comparison_matches_unknown_element_type() {
        let any = typed(ExprType::Array(None));
        let numbers = typed(ExprType::Array(Some(Box::new(ExprType::Number))));
        let texts: ExprValue = r#"["a"]"#.try_into().unwrap();
        assert!(any.try_eq(&texts).is_ok());
        assert!(numbers.try_eq(&texts).is_err());
    }

    #[test]
    fn expr_type_of_array_uses_the_type_of_its_elements() {
        let numbers: ExprValue = "[1, 2]".try_into().unwrap();
        assert_eq!(
            numbers.expr_type(),
            Some(ExprType::Array(Some(Box::new(ExprType::Number))))
        );
        assert_eq!(
            ExprValue::Array(vec![]).expr_type(),
            Some(ExprType::Array(None))
        );
        assert_eq!(ExprValue::Unknown(None).expr_type(), None);
    }
}
