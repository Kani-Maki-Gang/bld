#![allow(dead_code)]
use crate::config::definitions::DB_NAME;
use crate::path;
use crate::persist::{Execution, PipelineModel, run_migrations};
use anyhow::anyhow;
use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{Pool, PooledConnection, ConnectionManager};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::debug;

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
    pub pipeline: PipelineModel,
    pub connection: PooledConnection<ConnectionManager<SqliteConnection>>,
}

impl PipelineExecWrapper {
    pub fn new(pool: &Pool<ConnectionManager<SqliteConnection>>, pipeline: PipelineModel) -> anyhow::Result<Self> {
        Ok(Self {
            pipeline,
            connection: pool.get()?,
        })
    }
}

impl Execution for PipelineExecWrapper {
    fn update(&mut self, running: bool) -> anyhow::Result<()> {
        let end_date_time = match running {
            true => String::new(),
            false => chrono::Utc::now().to_string(),
        };
        PipelineModel::update(&self.connection, &self.pipeline.id, running, &end_date_time)?;
        self.pipeline.running = running;
        self.pipeline.end_date_time = end_date_time;
        debug!(
            "updated pipeline of id: {}, name: {} with new values running: {}, end_date_time: {}", 
            self.pipeline.id,
            self.pipeline.name,
            self.pipeline.running,
            self.pipeline.end_date_time
        );
        Ok(())
    }
}

pub struct NullExec;

impl NullExec {
    pub fn atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Execution for NullExec {
    fn update(&mut self, _running: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
