#![allow(dead_code)]
use crate::config::definitions::DB_NAME;
use crate::path;
use crate::persist::pipeline_runs::{self, PipelineRuns};
use crate::persist::{run_migrations, Execution};
use anyhow::anyhow;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::debug;

const EMPTY: String = String::new();

fn no_pipeline_instance() -> anyhow::Result<()> {
    Err(anyhow!("no pipeline instance"))
}

pub fn new_connection_pool(db: &str) -> anyhow::Result<Pool<ConnectionManager<SqliteConnection>>> {
    let path = path![db, DB_NAME].as_path().display().to_string();
    debug!("creating sqlite connection pool");
    let manager = ConnectionManager::<SqliteConnection>::new(path);
    let pool = Pool::builder().build(manager)?;
    debug!("running migrations");
    let conn = pool.get()?;
    run_migrations(&conn)?;
    Ok(pool)
}

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
            self.pipeline_run.end_date_time.as_ref().unwrap_or(&EMPTY)
        );
        Ok(())
    }
}

pub struct EmptyExec;

impl EmptyExec {
    pub fn atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Execution for EmptyExec {
    fn update_running(&mut self, _running: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
