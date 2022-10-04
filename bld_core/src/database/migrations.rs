use anyhow::Result;
use diesel::SqliteConnection;
use tracing::debug;

embed_migrations!();

pub fn run_migrations(conn: &SqliteConnection) -> Result<()> {
    embedded_migrations::run(conn)?;
    debug!("executed migrations");
    Ok(())
}
