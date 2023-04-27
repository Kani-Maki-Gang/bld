use anyhow::Result;

pub trait Load<T> {
    fn load(input: &str) -> Result<T>;
    fn load_with_verbose_errors(input: &str) -> Result<T>;
}
