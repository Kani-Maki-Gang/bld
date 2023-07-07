use anyhow::{anyhow, Result};
use diesel::{
    prelude::*, query_dsl::RunQueryDsl, sqlite::SqliteConnection, Connection, Insertable, Queryable,
};
use tracing::{debug, error};
use uuid::Uuid;

use crate::database::schema::cron_job_environment_variables;
use crate::database::schema::cron_job_environment_variables::dsl::*;

#[derive(Debug, Queryable)]
#[diesel(belongs_to(CronJob, foreign_key = cron_job_id))]
#[diesel(table_name = cron_job_environment_variables)]
pub struct CronJobEnvironmentVariable {
    pub id: String,
    pub name: String,
    pub value: String,
    pub cron_job_id: String,
    pub date_created: String,
    pub date_updated: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = cron_job_environment_variables)]
pub struct InsertCronJobEnvironmentVariable<'a> {
    pub id: String,
    pub name: &'a str,
    pub value: &'a str,
    pub cron_job_id: &'a str,
}

impl<'a> InsertCronJobEnvironmentVariable<'a> {
    pub fn new(kv: (&'a String, &'a String), job_id: &'a str) -> Self {
        let cve_id = Uuid::new_v4().to_string();
        let (cve_name, cve_value) = kv;
        Self {
            id: cve_id,
            name: cve_name,
            value: cve_value,
            cron_job_id: job_id,
        }
    }
}

pub fn select_by_cron_job_id(
    conn: &mut SqliteConnection,
    cev_cron_job_id: &str,
) -> Result<Vec<CronJobEnvironmentVariable>> {
    debug!("loading all environment variables for cron job with id: {cev_cron_job_id}");
    cron_job_environment_variables
        .filter(cron_job_id.eq(cev_cron_job_id))
        .load(conn)
        .map(|cev| {
            debug!("loaded cron job environment variables successfully");
            cev
        })
        .map_err(|e| {
            error!("couldn't load cron job environment variables due to {e}");
            anyhow!(e)
        })
}

pub fn insert_many(
    conn: &mut SqliteConnection,
    models: &[InsertCronJobEnvironmentVariable],
) -> Result<()> {
    conn.transaction(|conn| {
        diesel::insert_into(cron_job_environment_variables::table)
            .values(models)
            .execute(conn)
            .map(|_| {
                debug!("created new cron job environment variables successfully");
            })
            .map_err(|e| {
                error!("couldn't insert cron job environment variables due to {e}");
                anyhow!(e)
            })
    })
}

pub fn delete_by_cron_job_id(conn: &mut SqliteConnection, cev_cron_job_id: &str) -> Result<()> {
    debug!(
        "deleting cron job environment variables associated with cron job id: {cev_cron_job_id}"
    );
    diesel::delete(cron_job_environment_variables::table)
        .filter(cron_job_id.eq(cev_cron_job_id))
        .execute(conn)
        .map(|_| {
            debug!("deleted all cron job environment variables successfully");
        })
        .map_err(|e| {
            error!("couldn't delete cron job environment variables due to {e}");
            anyhow!(e)
        })
}
