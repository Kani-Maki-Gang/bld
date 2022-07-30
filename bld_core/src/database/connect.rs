use crate::database::run_migrations;
use bld_config::{definitions::DB_NAME, path};
use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Error, Pool};
use diesel::sqlite::SqliteConnection;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error};

#[derive(Debug)]
struct SqliteConnectionOptions {
    enable_wal: bool,
    enabld_foreign_keys: bool,
    busy_timeout: Option<Duration>,
}

impl SqliteConnectionOptions {
    fn customize(&self, conn: &mut SqliteConnection) -> Result<(), diesel::result::Error> {
        if self.enable_wal {
            conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
                .map_err(|e| {
                    error!("error trying to set wal option for connection. {e}");
                    e
                })?;
        }
        if self.enabld_foreign_keys {
            conn.batch_execute("PRAGMA foreign_keys = ON;")
                .map_err(|e| {
                    error!("error trying to set foreign keys option for connection. {e}");
                    e
                })?;
        }
        if let Some(duration) = self.busy_timeout {
            conn.batch_execute(&format!("PRAGMA busy_timeout = {};", duration.as_millis()))
                .map_err(|e| {
                    error!("error trying to set busy_timeout option for connection. {e}");
                    e
                })?;
        }
        Ok(())
    }
}

impl CustomizeConnection<SqliteConnection, Error> for SqliteConnectionOptions {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), Error> {
        self.customize(conn).map_err(Error::QueryError)
    }
}

pub fn new_connection_pool(db: &str) -> anyhow::Result<Pool<ConnectionManager<SqliteConnection>>> {
    let path = path![db, DB_NAME].as_path().display().to_string();
    debug!("creating sqlite connection pool");
    let pool = Pool::builder()
        .max_size(16)
        .connection_customizer(Box::new(SqliteConnectionOptions {
            enable_wal: true,
            enabld_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(30)),
        }))
        .build(ConnectionManager::<SqliteConnection>::new(path))?;
    debug!("running migrations");
    let conn = pool.get()?;
    run_migrations(&conn)?;
    Ok(pool)
}
