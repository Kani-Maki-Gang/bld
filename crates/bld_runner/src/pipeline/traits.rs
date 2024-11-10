use anyhow::Result;
use bld_config::BldConfig;

pub trait Load<T> {
    fn load(input: &str) -> Result<T>;
    fn load_with_verbose_errors(input: &str) -> Result<T>;
}

pub trait Dependencies {
    fn local_deps(&self, config: &BldConfig) -> Vec<String>;
}
