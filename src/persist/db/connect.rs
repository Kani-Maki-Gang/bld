#![allow(dead_code)]
use crate::config::definitions::DB_NAME;
use crate::path;
use crate::persist::pipeline::{self, Pipeline};
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
    pub pipeline: Pipeline,
    pub connection: PooledConnection<ConnectionManager<SqliteConnection>>,
}

impl PipelineExecWrapper {
    pub fn new(
        pool: &Pool<ConnectionManager<SqliteConnection>>,
        pipeline: Pipeline,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            pipeline,
            connection: pool.get()?,
        })
    }
}

impl Execution for PipelineExecWrapper {
    fn update(&mut self, running: bool) -> anyhow::Result<()> {
        self.pipeline = pipeline::update(&self.connection, &self.pipeline.id, running)?;
        debug!(
            "updated pipeline of id: {}, name: {} with new values running: {}, end_date_time: {}",
            self.pipeline.id,
            self.pipeline.name,
            self.pipeline.running,
            self.pipeline.end_date_time.as_ref().unwrap_or(&EMPTY)
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
    fn update(&mut self, _running: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
