use anyhow::Result;

pub trait SyntaxTree {}

pub trait ApplyContext {
    fn apply_context(&self) -> Result<()>;
}

pub trait Parse<Tree> {
    fn parse(node: &Self) -> Result<Tree>;
}

pub trait Load<Tree> {
    fn load(input: &str) -> Result<Tree>;
}

pub trait Validate {
    fn validate(&self) -> Result<()>;
}
