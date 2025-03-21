mod action;
mod common;
mod job;
mod pipeline;

pub use action::*;
use anyhow::Result;
pub use pipeline::*;

pub enum FileRunner {
    Action(ActionRunner),
    Pipeline(PipelineRunner),
}

impl FileRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Action(runner) => runner.run().await,
            Self::Pipeline(runner) => runner.run().await.await,
        }
    }
}
