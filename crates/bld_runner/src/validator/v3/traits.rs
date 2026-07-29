use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;

use crate::inputs::v3::Input;

/// For files that don't define an env section, such as actions.
pub static EMPTY_ENV: LazyLock<HashMap<String, String>> = LazyLock::new(HashMap::new);

/// The point in time a value is used, which defines the symbols its expressions can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprScope {
    /// The value is used before any step has run, meaning only the symbols of the readonly
    /// context are available. Input defaults, env values, everything under `runs_on` and a
    /// job's `if` and `strategy` are all resolved at that point.
    StartOfRun,
    /// The value is used while the run is under way, so step outputs and matrix values are
    /// available as well.
    Runtime,
}

pub trait ValidatorContext<'a> {
    fn get_config(&self) -> Arc<BldConfig>;
    fn get_fs(&self) -> Arc<FileSystem>;
    fn get_package_manager(&self) -> Arc<PackageManager>;
    fn push_section(&mut self, section: &'a str);
    fn push_job_section(&mut self, section: &'a str);
    fn pop_section(&mut self);
    #[allow(dead_code)]
    fn clear_section(&mut self);
    fn append_error(&mut self, error: &str);
    fn expression_count(&self, value: &str) -> usize;
    fn contains_expressions(&mut self, value: &str) -> bool;
    fn validate_expressions(&mut self, symbol: &'a str, scope: ExprScope);
    fn validate_array_expression(&mut self, symbol: &'a str, scope: ExprScope);
    fn matrix_refs(&self, value: &str) -> Vec<String>;
    fn validate_file_path(&mut self, value: &'a str);
    fn validate_env(&mut self, env: &'a HashMap<String, String>, scope: ExprScope);
    /// Checks that the values needed before the run starts, meaning the defaults of the
    /// file's inputs and its env values, can actually be resolved at that point.
    fn validate_start_of_run_values(
        &mut self,
        inputs: &'a HashMap<String, Input>,
        env: &'a HashMap<String, String>,
    );
}

pub trait ConsumeValidator {
    async fn validate(self) -> Result<()>;
}

pub trait Validate<'a> {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C);
}
