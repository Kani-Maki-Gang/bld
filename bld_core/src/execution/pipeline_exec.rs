use crate::database::pipeline_runs::{self, PipelineRuns};
use crate::execution::Execution;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use tracing::debug;

const EMPTY_STRING: String = String::new();

pub struct PipelineExecWrapper {
    pub pipeline_run: PipelineRuns,
    pub connection: PooledConnection<ConnectionManager<SqliteConnection>>,
}

impl PipelineExecWrapper {
    pub fn new(
        pool: &Pool<ConnectionManager<SqliteConnection>>,
        pipeline_run: PipelineRuns,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            pipeline_run,
            connection: pool.get()?,
        })
    }
}

impl Execution for PipelineExecWrapper {
    fn update_running(&mut self, running: bool) -> anyhow::Result<()> {
        self.pipeline_run =
            pipeline_runs::update_running(&self.connection, &self.pipeline_run.id, running)?;
        debug!(
            "updated pipeline run of id: {}, name: {} with new values running: {}, end_date_time: {}",
            self.pipeline_run.id,
            self.pipeline_run.name,
            self.pipeline_run.running,
            self.pipeline_run.end_date_time.as_ref().unwrap_or(&EMPTY_STRING)
        );
        Ok(())
    }
}
