use crate::database::pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_RUNNING};
use anyhow::{bail, Result};
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

impl Execution {
    pub fn empty_atom() -> Arc<Self> {
        Arc::new(Self::Empty)
    }

    pub fn pipeline_atom(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: &str,
    ) -> Arc<Self> {
        Arc::new(Self::Pipeline {
            pool,
            run_id: run_id.to_string(),
        })
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

    pub fn check_stop_signal(&self) -> Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Pipeline { pool, run_id } => {
                let mut conn = pool.get()?;
                pipeline_runs::select_by_id(&mut conn, run_id).and_then(|r| match r.stopped {
                    Some(true) => bail!(""),
                    _ => Ok(()),
                })
            }
        }
    }
}
