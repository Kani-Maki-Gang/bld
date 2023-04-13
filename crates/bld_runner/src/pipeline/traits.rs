use std::collections::HashMap;

use anyhow::Result;

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

pub trait StaticTokenTransformer<'a, T, V> {
    fn transform(&self, text: String, context: &V) -> String
    where
        V: StaticTokenContext<'a, T>;
}

pub trait DynamicTokenTransformer<'a, T, V> {
    fn transform(&self, text: String, context: &V) -> String
    where
        V: DynamicTokenContext<'a, T>;
}

pub trait ApplyStaticTokens<'a, T, V> {
    fn apply_tokens(&mut self, context: &T) -> Result<Self>
    where
        Self: Sized,
        T: StaticTokenContext<'a, V>;
}
