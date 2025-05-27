#![allow(dead_code)]

use std::{fmt::Display, iter::Peekable};

use anyhow::{Result, bail};
use pest::iterators::{Pair, Pairs};

use super::parser::Rule;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub enum ExprText<'a> {
    Ref(&'a str),
    Owned(String),
}

#[derive(Debug)]
pub enum ExprValue<'a> {
    Boolean(bool),
    Number(f64),
    Text(ExprText<'a>),
}

impl<'a, 'b> ExprValue<'a> {
    pub fn type_as_string(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Number(_) => "number",
            Self::Text(_) => "text",
        }
    }

    pub fn try_eq(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        let value = match (self, other) {
            (Self::Boolean(l), Self::Boolean(r)) => l == r,
            (Self::Number(l), Self::Number(r)) => l == r,
            (Self::Text(l), Self::Text(r)) => l == r,
            _ => bail!(
                "cannot compare {} and {}",
                self.type_as_string(),
                other.type_as_string()
            ),
        };
        Ok(ExprValue::<'b>::Boolean(value))
    }

    pub fn try_ord(&self, other: &'a Self) -> Result<ExprValue<'b>> {
        let value = match (self, other) {
            (Self::Number(l), Self::Number(r)) => l > r,
            (Self::Text(l), Self::Text(r)) => l > r,
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
        if let Ok(num) = value.parse::<f64>() {
            return Ok(ExprValue::Number(num));
        }

        if let Ok(boolean) = value.parse::<bool>() {
            return Ok(ExprValue::Boolean(boolean));
        }

        let mut text = String::new();
        if value.starts_with("\\\"") {
            text = value.replace("\\\"", "");
        }

        if value.starts_with("\"") {
            text = value.replace("\"", "");
        }

        if value.ends_with("\\\"") {
            text = value.replace("\\\"", "");
        }

        if value.ends_with("\"") {
            text = value.replace("\"", "");
        }

        Ok(ExprValue::Text(ExprText::Owned(text)))
    }
}

impl Display for ExprValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Boolean(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::Text(ExprText::Ref(value)) => value.to_string(),
            Self::Text(ExprText::Owned(value)) => value.to_string(),
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

pub trait WritableRuntimeExprContext {
    fn get_output(&self, name: &str) -> Result<&str>;
    fn set_output(&mut self, name: String, value: String) -> Result<()>;
}

pub trait EvalObject<'a> {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>>;
}

pub trait EvalExpr<'a> {
    fn eval_cmp(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval_symbol(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval_expr(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval_logical_expr(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval(&'a self, expr: &'a str) -> Result<ExprValue<'a>>;
}
