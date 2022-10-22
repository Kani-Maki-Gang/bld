use crate::database::run_migrations;
use anyhow::Result;
use bld_config::definitions::DB_NAME;
use bld_config::path;
use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Error, Pool};
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use std::fmt::Write;
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
    fn customize(&self, conn: &mut SqliteConnection) -> Result<(), DieselError> {
        let mut pragma = String::new();
        if self.enable_wal {
            let _ = write!(
                pragma,
                "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;"
            );
        }
        if self.enabld_foreign_keys {
            let _ = write!(pragma, "PRAGMA foreign_keys = ON;");
        }
        if let Some(duration) = self.busy_timeout {
            let _ = write!(pragma, "PRAGMA busy_timeout = {};", duration.as_millis());
        }
        conn.batch_execute(&pragma).map_err(|e| {
            error!("error trying to set busy_timeout option for connection. {e}");
            e
        })
    }
}

impl CustomizeConnection<SqliteConnection, Error> for SqliteConnectionOptions {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), Error> {
        self.customize(conn).map_err(Error::QueryError)
    }
}

pub fn new_connection_pool(db: &str) -> Result<Pool<ConnectionManager<SqliteConnection>>> {
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
    let mut conn = pool.get()?;
    run_migrations(&mut conn)?;
    Ok(pool)
}
