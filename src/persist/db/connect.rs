#![allow(dead_code)]
use crate::definitions::DB_NAME;
use crate::path;
use crate::persist::Execution;
use crate::persist::PipelineModel;
use crate::types::{BldError, Result};
use diesel::sqlite::SqliteConnection;
use diesel::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn no_pipeline_instance() -> Result<()> {
    let message = String::from("no pipeline instance");
    Err(BldError::Other(message))
}

pub struct Database {
    pub pipeline: Option<PipelineModel>,
    connection: SqliteConnection,
}

impl Database {
    fn initialize(conn: &SqliteConnection) -> Result<()> {
        PipelineModel::create(conn)?;
        Ok(())
    }

    pub fn connect(db: &str) -> Result<Self> {
        let path_buf = path![db, DB_NAME];
        let path_str = path_buf.as_path().display().to_string();
        let is_new = !path_buf.is_file();
        let connection = SqliteConnection::establish(&path_str)?;
        if is_new {
            Database::initialize(&connection)?;
        }
        Ok(Self {
            connection,
            pipeline: None,
        })
    }

    pub fn all(&self) -> Result<Vec<PipelineModel>> {
        PipelineModel::select_all(&self.connection)
    }

    pub fn load(&mut self, id: &str) {
        self.pipeline = PipelineModel::select_by_id(&self.connection, &id);
    }

    pub fn add(&mut self, id: &str, name: &str) -> Result<()> {
        let pipeline = PipelineModel {
            id: id.to_string(),
            name: name.to_string(),
            running: false,
        };
        PipelineModel::insert(&self.connection, &pipeline)?;
        self.pipeline = Some(pipeline);
        Ok(())
    }
}

impl Execution for Database {
    fn update(&mut self, running: bool) -> Result<()> {
        match self.pipeline.as_mut() {
            Some(mut pipeline) => {
                PipelineModel::update(&self.connection, &pipeline.id, running)?;
                pipeline.running = running;
                Ok(())
            }
            None => no_pipeline_instance(),
        }
    }
}

pub struct NullExec;

impl NullExec {
    pub fn atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Execution for NullExec {
    fn update(&mut self, _running: bool) -> Result<()> {
        Ok(())
    }
}
