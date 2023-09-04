use anyhow::{anyhow, Result};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::debug;

use super::DbConnection;

const SQLITE_EMBEDDED_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");
const POSTGRES_EMBEDDED_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/postgres");

pub fn run_migrations(conn: &mut DbConnection) -> Result<()> {
    let migrations = match conn {
        DbConnection::Sqlite(_) => SQLITE_EMBEDDED_MIGRATIONS,
        DbConnection::Postgres(_) => POSTGRES_EMBEDDED_MIGRATIONS,
        _ => unreachable!(),
    };
    conn.run_pending_migrations(migrations)
        .map_err(|e| anyhow!(e))?;
    debug!("executed migrations");
    Ok(())
}
