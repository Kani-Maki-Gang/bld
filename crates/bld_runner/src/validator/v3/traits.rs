use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;

pub trait ValidatorContext<'a> {
    fn get_config(&self) -> Arc<BldConfig>;
    fn get_fs(&self) -> Arc<FileSystem>;
    fn push_section(&mut self, section: &'a str);
    fn pop_section(&mut self);
    #[allow(dead_code)]
    fn clear_section(&mut self);
    fn append_error(&mut self, error: &str);
    fn contains_symbols(&mut self, value: &str) -> bool;
    fn validate_symbols(&mut self, symbol: &'a str);
    fn validate_keywords(&mut self, name: &'a str);
    fn validate_file_path(&mut self, value: &'a str);
    fn validate_env(&mut self, env: &'a HashMap<String, String>);
}

pub trait ConsumeValidator {
    async fn validate(self) -> Result<()>;
}

pub trait Validate<'a> {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C);
}
