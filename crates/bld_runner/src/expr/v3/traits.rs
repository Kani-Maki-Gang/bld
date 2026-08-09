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
    Number(f64),
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
            Self::Number(_) => "number",
            Self::Text(_) => "text",
            Self::Array(_) => "array",
            Self::Unknown => "unknown",
        }
    }

    pub fn try_eq(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Ok(ExprValue::<'b>::Boolean(true));
        }

        let value = match (self, other) {
            (Self::Boolean(l), Self::Boolean(r)) => l == r,
            (Self::Number(l), Self::Number(r)) => l == r,
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
            (Self::Number(l), Self::Number(r)) => l > r,
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
        if let Ok(num) = value.parse::<f64>() {
            return Ok(ExprValue::Number(num));
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
            Self::Number(value) => value.to_string(),
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
    fn get_output<'a>(&'a self, id: &str, name: &str) -> Result<ExprValue<'a>>;
    fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()>;
    fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()>;
    #[allow(clippy::needless_lifetimes)]
    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<&'a str>;
    fn get_job_output<'a>(&'a self, job: &str, name: &str) -> Result<ExprValue<'a>>;
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
