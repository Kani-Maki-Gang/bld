use crate::runner::v1;
use crate::runner::v2;
use crate::runner::v3;
use anyhow::Result;

pub enum VersionedPipelineRunner {
    V1(v1::Runner),
    V2(v2::Runner),
    V3(v3::PipelineRunner),
}

impl VersionedPipelineRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::V1(runner) => runner.run().await.await,
            Self::V2(runner) => runner.run().await.await,
            Self::V3(runner) => runner.run().await.await,
        }
    }
}

pub enum VersionedActionRunner {
    V3(v3::ActionRunner),
}

impl VersionedActionRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::V3(runner) => runner.run().await,
        }
    }
}
