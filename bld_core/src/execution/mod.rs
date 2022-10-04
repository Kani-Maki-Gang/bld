use crate::database::pipeline_runs::{self, PR_STATE_FINISHED, PR_STATE_RUNNING};
use anyhow::{bail, Result};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::{Arc, Mutex};

pub enum Execution {
    Empty,
    Pipeline {
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: String,
    },
}

impl Execution {
    pub fn empty_atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::Empty))
    }

    pub fn pipeline_atom(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        run_id: &str,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::Pipeline {
            pool,
            run_id: run_id.to_string(),
        }))
    }

    fn update_state(&mut self, state: &str) -> Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Pipeline { pool, run_id } => {
                let conn = pool.get()?;
                pipeline_runs::update_state(&conn, run_id, state).map(|_| ())
            }
        }
    }

    pub fn set_as_running(&mut self) -> Result<()> {
        self.update_state(PR_STATE_RUNNING)
    }

    pub fn set_as_finished(&mut self) -> Result<()> {
        self.update_state(PR_STATE_FINISHED)
    }

    pub fn check_stop_signal(&self) -> Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Pipeline { pool, run_id } => {
                let conn = pool.get()?;
                pipeline_runs::select_by_id(&conn, run_id).and_then(|r| match r.stopped {
                    Some(true) => bail!(""),
                    _ => Ok(()),
                })
            }
        }
    }
}
