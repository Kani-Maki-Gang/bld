use anyhow::Result;

pub trait ExecutionContext {
    async fn transform(&self, text: String) -> Result<String>;
}

pub trait ApplyContext {
    async fn apply_context<C: ExecutionContext>(&mut self, context: &C) -> Result<()>;
}
