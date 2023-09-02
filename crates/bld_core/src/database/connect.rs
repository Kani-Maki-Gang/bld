use anyhow::Result;
use diesel::{
    connection::SimpleConnection,
    mysql::MysqlConnection,
    pg::PgConnection,
    r2d2::{CustomizeConnection, Error},
    result::Error as DieselError,
    sqlite::SqliteConnection,
    MultiConnection,
};
use std::{fmt::Write, time::Duration};
use tracing::error;

#[derive(MultiConnection)]
pub enum DbConnection {
    Sqlite(SqliteConnection),
    Postgres(PgConnection),
    Mysql(MysqlConnection),
}

#[derive(Debug)]
pub struct DbConnectionOptions {
    pub enable_wal: bool,
    pub enabld_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl DbConnectionOptions {
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

impl CustomizeConnection<DbConnection, Error> for DbConnectionOptions {
    fn on_acquire(&self, conn: &mut DbConnection) -> Result<(), Error> {
        let DbConnection::Sqlite(conn) = conn else {
            return Ok(());
        };
        self.customize(conn).map_err(Error::QueryError)
    }
}
