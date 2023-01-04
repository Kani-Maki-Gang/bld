use crate::database::pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_RUNNING};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;

pub enum Execution {
    Empty,
    Pipeline {
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: String,
    },
}

impl Default for Execution {
    fn default() -> Self {
        Self::Empty
    }
}

impl Execution {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, run_id: &str) -> Self {
        Self::Pipeline {
            pool,
            run_id: run_id.to_string(),
        }
    }

    fn update_state(&self, state: &str) -> Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Pipeline { pool, run_id } => {
                let mut conn = pool.get()?;
                pipeline_runs::update_state(&mut conn, run_id, state).map(|_| ())
            }
        }
    }

    pub fn set_as_running(&self) -> Result<()> {
        self.update_state(PR_STATE_RUNNING)
    }

    pub fn set_as_finished(&self) -> Result<()> {
        self.update_state(PR_STATE_FINISHED)
    }

    pub fn set_as_faulted(&self) -> Result<()> {
        self.update_state(PR_STATE_FAULTED)
    }
}
