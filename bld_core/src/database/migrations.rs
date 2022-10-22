use anyhow::{anyhow, Result};
use diesel::SqliteConnection;
use tracing::debug;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

const EMBEDDED_MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(conn: &SqliteConnection) -> Result<()> {
    conn.run_pending_migrations(EMBEDDED_MIGRATIONS).map_err(|e| anyhow!(e))?;
    debug!("executed migrations");
    Ok(())
}
