mod connect;
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
mod migrations;
pub mod pipeline;
pub mod pipeline_run_containers;
pub mod pipeline_runs;
mod schema;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use bld_config::BldConfig;
pub use connect::*;
use diesel::r2d2::{ConnectionManager, Pool};
pub use migrations::*;
pub use schema::*;
use tracing::debug;

pub fn new_connection_pool(
    config: Arc<BldConfig>,
) -> Result<Pool<ConnectionManager<DbConnection>>> {
    let path = config.db_full_path().display().to_string();

    debug!("creating sqlite connection pool");

    let pool = Pool::builder()
        .max_size(16)
        .connection_customizer(Box::new(DbConnectionOptions {
            enable_wal: true,
            enabld_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(30)),
        }))
        .build(ConnectionManager::<DbConnection>::new(path))?;

    debug!("running migrations");
    let mut conn = pool.get()?;
    run_migrations(&mut conn)?;

    Ok(pool)
}
