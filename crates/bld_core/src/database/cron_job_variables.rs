use anyhow::{anyhow, Result};
use diesel::{query_dsl::RunQueryDsl, sqlite::SqliteConnection, Connection, Insertable, Queryable};
use tracing::{debug, error};

use crate::database::schema::cron_job_variables;
use crate::database::schema::cron_job_variables::dsl::*;

#[derive(Debug, Queryable)]
#[diesel(belongs_to(CronJob, foreign_key = cron_job_id))]
#[diesel(table_name = cron_job_variables)]
pub struct CronJobVariable {
    pub id: i32,
    pub name: String,
    pub value: String,
    pub cron_job_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = cron_job_variables)]
pub struct InsertCronJobVariable<'a> {
    pub id: i32,
    pub name: &'a str,
    pub value: &'a str,
    pub cron_job_id: i32,
}

pub fn select_by_cron_job_id(
    conn: &mut SqliteConnection,
    cev_cron_job_id: i32,
) -> Result<Vec<CronJobVariable>> {
    debug!("loading all variables for cron job with id: {cev_cron_job_id}");
    cron_job_variables
        .load(conn)
        .map(|cev| {
            debug!("loaded cron job variables successfully");
            cev
        })
        .map_err(|e| {
            error!("couldn't load cron job variables due to {e}");
            anyhow!(e)
        })
}

pub fn insert_many(conn: &mut SqliteConnection, models: &[InsertCronJobVariable]) -> Result<()> {
    conn.transaction(|conn| {
        diesel::insert_into(cron_job_variables::table)
            .values(models)
            .execute(conn)
            .map(|_| {
                debug!("created new cron job environment variables successfully");
                ()
            })
            .map_err(|e| {
                error!("couldn't insert cron job environment variables due to {e}");
                anyhow!(e)
            })
    })
}
