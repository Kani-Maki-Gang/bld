mod action;
mod common;
mod job;
mod pipeline;
mod state;
#[cfg(test)]
mod test_utils;

pub use action::*;
pub use pipeline::*;
pub use state::*;

use std::collections::HashMap;

use anyhow::Result;

pub enum FileRunner {
    Action(Box<ActionRunner<ActionState>>),
    Pipeline(Box<PipelineRunner>),
}

impl FileRunner {
    pub async fn run(self) -> Result<HashMap<String, String>> {
        match self {
            Self::Action(runner) => runner.run().await,
            Self::Pipeline(runner) => runner.run().await.await,
        }
    }
}
