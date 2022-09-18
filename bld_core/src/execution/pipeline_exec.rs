use crate::database::pipeline_runs;
use crate::execution::Execution;
use anyhow::bail;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::{Arc, Mutex};

pub struct PipelineExecution {
    pub pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    pub run_id: String,
}

impl PipelineExecution {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, run_id: &str) -> Self {
        Self {
            pool,
            run_id: run_id.to_string(),
        }
    }

    pub fn atom(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: &str,
    ) -> anyhow::Result<Arc<Mutex<Self>>> {
        Ok(Arc::new(Mutex::new(Self::new(pool, run_id))))
    }
}

impl Execution for PipelineExecution {
    fn update_state(&mut self, state: &str) -> anyhow::Result<()> {
        let conn = self.pool.get()?;
        pipeline_runs::update_state(&conn, &self.run_id, state).map(|_| ())
    }

    fn check_stop_signal(&self) -> anyhow::Result<()> {
        let conn = self.pool.get()?;
        pipeline_runs::select_by_id(&conn, &self.run_id).and_then(|r| match r.stopped {
            Some(true) => bail!(""),
            _ => Ok(()),
        })
    }
}
