#![allow(dead_code)]

use std::iter::Peekable;

use anyhow::{Result, bail};
use pest::iterators::{Pair, Pairs};

use super::parser::Rule;

#[derive(Eq, Ord, PartialOrd, PartialEq)]
pub enum ExprText<'a> {
    Ref(&'a str),
    Owned(String),
}

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

        Ok(ExprValue::Text(ExprText::Owned(value.to_string())))
    }
}

pub trait RuntimeExecutionContext<'a> {
    fn get_root_dir(&self) -> &'a str;
    fn get_project_dir(&self) -> &'a str;
    fn get_input(&'a self, name: &'a str) -> Result<&'a str>;
    fn get_output(&self, name: &'a str) -> Result<&'a str>;
    fn set_output(&mut self, name: &'a str, value: &'a str) -> Result<()>;
    fn get_env(&'a self, name: &'a str) -> Result<&'a str>;
    fn get_run_id(&self) -> &'a str;
    fn get_run_start_time(&self) -> &'a str;
}

pub trait EvalObject<'a> {
    fn eval_object<Ctx: RuntimeExecutionContext<'a>>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        ctx: &Ctx,
    ) -> Result<ExprValue<'a>>;
}

pub trait EvalExpr<'a> {
    fn eval_cmp(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval_symbol(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval_expr(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval_logical_expr(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>>;
    fn eval(&'a mut self, expr: &'a str) -> Result<ExprValue<'a>>;
}
