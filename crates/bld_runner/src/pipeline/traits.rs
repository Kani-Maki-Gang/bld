use std::collections::HashMap;

use anyhow::Result;

use crate::keywords::version2::{BldDirectory, Environment, RunId, RunStartTime, Variable};

pub trait Load<T> {
    fn load(input: &str) -> Result<T>;
    fn load_with_verbose_errors(input: &str) -> Result<T>;
}

pub trait StaticTokenContext<'a, T> {
    fn retrieve(&'a self) -> &'a str;
}

pub trait DynamicTokenContext<'a, T> {
    fn retrieve(&'a self) -> HashMap<String, String>;
}

pub trait StaticTokenTransformer<'a, T>: StaticTokenContext<'a, T> {
    fn transform(&'a self, text: String) -> String;
}

pub trait DynamicTokenTransformer<'a, T>: DynamicTokenContext<'a, T> {
    fn transform(&'a self, text: String) -> String;
}

pub trait HolisticTokenTransformer<'a>:
    StaticTokenTransformer<'a, BldDirectory>
    + DynamicTokenTransformer<'a, Variable>
    + DynamicTokenTransformer<'a, Environment>
    + StaticTokenTransformer<'a, RunId>
    + StaticTokenTransformer<'a, RunStartTime>
{
    fn transform(&'a self, text: String) -> String;
}

pub trait ApplyTokens<'a, T> {
    fn apply_tokens(&mut self, context: &'a T) -> Result<()>
    where
        Self: Sized,
        T: StaticTokenTransformer<'a, BldDirectory>
            + DynamicTokenTransformer<'a, Variable>
            + DynamicTokenTransformer<'a, Environment>
            + StaticTokenTransformer<'a, RunId>
            + StaticTokenTransformer<'a, RunStartTime>;
}
