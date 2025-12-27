use std::collections::HashMap;

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;

#[allow(async_fn_in_trait)]
pub trait Load<T> {
    async fn load(&self, path: &str) -> Result<T>;
    fn load_with_verbose_errors(&self, path: &str) -> Result<T>;
}

#[allow(async_fn_in_trait)]
pub trait Dependencies {
    async fn local_deps(&self, config: &BldConfig, fs: &FileSystem) -> Vec<String>;
}

#[allow(async_fn_in_trait)]
pub trait Validate {
    async fn validate(self) -> Result<()>;
}

pub type Variables = (
    Option<HashMap<String, String>>,
    Option<HashMap<String, String>>,
);

pub trait IntoVariables {
    fn into_variables(self) -> Variables;
}
