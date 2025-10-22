mod action;
mod common;
mod job;
mod pipeline;
mod state;

pub use action::*;
pub use pipeline::*;
pub use state::*;

use anyhow::Result;

pub enum FileRunner {
    Action(Box<ActionRunner<ActionState>>),
    Pipeline(Box<PipelineRunner>),
}

impl FileRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Action(runner) => runner.run().await,
            Self::Pipeline(runner) => runner.run().await.await,
        }
    }
}
