use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::keywords::version2::{BldDirectory, Environment, RunId, RunStartTime, Variable};

pub trait Load<T> {
    fn load(input: &str) -> Result<T>;
    fn load_with_verbose_errors(input: &str) -> Result<T>;
}

pub trait TokenContext<'a, T, V> {
    fn retrieve(&'a self) -> V;
}

#[async_trait]
pub trait TokenTransformer<'a, T, V>: TokenContext<'a, T, V> {
    async fn transform(&'a self, text: String) -> Result<String>;
}

#[async_trait]
pub trait CompleteTokenTransformer<'a>:
    TokenTransformer<'a, BldDirectory, &'a str>
    + TokenTransformer<'a, Variable, &'a HashMap<String, String>>
    + TokenTransformer<'a, Environment, &'a HashMap<String, String>>
    + TokenTransformer<'a, RunId, &'a str>
    + TokenTransformer<'a, RunStartTime, &'a str>
{
    async fn transform(&'a self, text: String) -> Result<String>;
}

#[async_trait]
pub trait ApplyTokens<'a, T> {
    async fn apply_tokens(&mut self, context: &'a T) -> Result<()>
    where
        Self: Sized,
        T: CompleteTokenTransformer<'a>;
}
