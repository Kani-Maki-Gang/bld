use anyhow::{anyhow, Result};
use diesel::{query_dsl::RunQueryDsl, sqlite::SqliteConnection, Queryable};
use tracing::{debug, error};

use crate::database::schema::cron_jobs::dsl::*;

#[derive(Debug, Queryable)]
pub struct CronJob {
    pub id: i32,
    pub pipeline_id: String,
    pub schedule: String,
}

pub struct InsertCronJob<'a> {
    pub id: i32,
    pub pipeline_id: &'a str,
    pub schedule: &'a str,
}

pub fn select_all(conn: &mut SqliteConnection) -> Result<Vec<CronJob>> {
    debug!("loading all cron jobs from the database");
    cron_jobs
        .load(conn)
        .map(|c| {
            debug!("loaded all cron jobs successfully");
            c
        })
        .map_err(|e| {
            error!("couln't load cron jobs due to {e}");
            anyhow!(e)
        })
}
