use crate::runner::v1;
use crate::runner::v2;
use crate::runner::v3;
use anyhow::Result;

pub enum VersionedRunner {
    V1(Box<v1::Runner>),
    V2(Box<v2::Runner>),
    V3(v3::FileRunner),
}

impl VersionedRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::V1(runner) => runner.run().await.await,
            Self::V2(runner) => runner.run().await.await,
            Self::V3(runner) => runner.run().await,
        }
    }
}
