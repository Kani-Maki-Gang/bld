pub mod cron_job_environment_variables;
pub mod cron_job_variables;
pub mod cron_jobs;
pub mod ha_client_serial_responses;
pub mod ha_client_status;
pub mod ha_hard_state;
pub mod ha_log;
pub mod ha_members;
pub mod ha_members_after_consensus;
pub mod ha_snapshot;
pub mod ha_state_machine;
pub mod pipeline;
pub mod pipeline_run_containers;
pub mod pipeline_runs;

use std::sync::Arc;

use anyhow::{bail, Result};
use bld_config::BldConfig;
use bld_migrations::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tracing::debug;

pub async fn new_connection_pool(config: Arc<BldConfig>) -> Result<DatabaseConnection> {
    let Some(path) = &config.local.server.db else {
        bail!("No database connection uri in config");
    };

    debug!("creating sqlite connection pool");
    let conn = Database::connect(path).await?;

    debug!("running migrations");
    Migrator::up(&conn, None).await?;

    Ok(conn)
}
