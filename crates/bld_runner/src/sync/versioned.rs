use super::runner::RunnerV1;
use anyhow::Result;

pub enum VersionedRunner {
    Version1(RunnerV1),
}

impl VersionedRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Version1(runner) => runner.run().await.await,
        }
    }
}
