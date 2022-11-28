use anyhow::Result;

pub trait Load<T> {
    fn load(input: &str) -> Result<T>;
}

pub trait Validate {
    fn validate_root(&self) -> Result<()>;

    fn validate<T>(&self, parent: &T) -> Result<()>;
}

pub trait ApplyContext {
    fn apply_context(&self) -> Result<()>;
}

pub trait Executable {
    fn execute(&self) -> Result<()>;
}

pub trait Disposable {
    fn dispose(&self) -> Result<()>;
}
