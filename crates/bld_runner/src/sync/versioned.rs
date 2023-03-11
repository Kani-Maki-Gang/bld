use crate::runner::version1;
use crate::runner::version2;
use anyhow::Result;

pub enum VersionedRunner {
    Version1(version1::Runner),
    Version2(version2::Runner),
}

impl VersionedRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Version1(runner) => runner.run().await.await,
            Self::Version2(runner) => runner.run().await.await,
        }
    }
}
