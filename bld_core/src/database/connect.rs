use crate::database::run_migrations;
use bld_config::{definitions::DB_NAME, path};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::path::PathBuf;
use tracing::debug;

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
