pub mod dtos;
mod generated;
mod queries;

pub use queries::*;

use anyhow::{bail, Result};
use bld_config::BldConfig;
use bld_migrations::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::sync::Arc;
use tracing::debug;

pub async fn new_connection_pool(config: Arc<BldConfig>) -> Result<DatabaseConnection> {
    let Some(path) = &config.local.server.db else {
        bail!("No database connection uri in config");
    };

    debug!("creating sqlite connection pool");
    let mut options = ConnectOptions::new(path);
    options.max_connections(100).min_connections(5);

    let conn = Database::connect(options).await?;

    debug!("running migrations");
    Migrator::up(&conn, None).await?;

    Ok(conn)
}
