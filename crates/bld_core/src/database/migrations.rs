use anyhow::{anyhow, Result};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::debug;

use super::DbConnection;

const EMBEDDED_MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(conn: &mut DbConnection) -> Result<()> {
    conn.run_pending_migrations(EMBEDDED_MIGRATIONS)
        .map_err(|e| anyhow!(e))?;
    debug!("executed migrations");
    Ok(())
}
