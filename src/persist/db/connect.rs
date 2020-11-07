#![allow(dead_code)]
use crate::definitions::DB_NAME;
use crate::persist::Execution;
use crate::persist::PipelineModel;
use diesel::sqlite::SqliteConnection;
use diesel::Connection;
use std::io::{self, Error, ErrorKind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Database {
    pub pipeline: Option<PipelineModel>,
    connection: SqliteConnection,
}

impl Database {
    fn path(db: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(db);
        path.push(DB_NAME);
        path
    }

    fn initialize(conn: &SqliteConnection) -> io::Result<()> {
        PipelineModel::create(conn)?;
        Ok(())
    }

    pub fn connect(db: &str) -> io::Result<Self> {
        let path_buf = Database::path(db);
        let path_str = path_buf.as_path().display().to_string();
        let is_new = !path_buf.is_file();
        let connection = match SqliteConnection::establish(&path_str) {
            Ok(connection) => connection,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        if is_new {
            Database::initialize(&connection)?;
        }
        Ok(Self {
            connection,
            pipeline: None,
        })
    }

    pub fn load(&mut self, id: &str) {
        self.pipeline = PipelineModel::select_by_id(&self.connection, &id);
    }

    pub fn add(&mut self, id: &str, name: &str) -> io::Result<()> {
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
    fn update(&mut self, running: bool) -> io::Result<()> {
        match self.pipeline.as_mut() {
            Some(mut pipeline) => {
                PipelineModel::update(&self.connection, &pipeline.id, running)?;
                pipeline.running = running;
                Ok(())
            }
            None => Err(Error::new(ErrorKind::Other, "no pipeline instance")),
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
    fn update(&mut self, _running: bool) -> io::Result<()> {
        Ok(())
    }
}
