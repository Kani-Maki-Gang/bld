use std::collections::HashMap;

use anyhow::Result;
use bld_config::BldConfig;

pub trait Load<T> {
    fn load(input: &str) -> Result<T>;
    fn load_with_verbose_errors(input: &str) -> Result<T>;
}

pub trait Dependencies {
    fn local_deps(&self, config: &BldConfig) -> Vec<String>;
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
