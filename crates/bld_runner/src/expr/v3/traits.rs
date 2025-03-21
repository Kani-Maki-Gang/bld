#![allow(dead_code)]

use anyhow::Result;
use pest::iterators::Pairs;

use super::parser::Rule;

pub enum ExprValue<'a> {
    Boolean(bool),
    Number(f64),
    Text(&'a str),
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
    fn eval_object(&'a self, path: &mut Pairs<'_, Rule>) -> Result<ExprValue<'a>>;
}

pub trait EvalExpr<'a> {
    fn eval(&'a mut self, expr: &'a str) -> Result<ExprValue<'a>>;
}
