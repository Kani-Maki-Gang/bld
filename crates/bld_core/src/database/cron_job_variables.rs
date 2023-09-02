use anyhow::{anyhow, Result};
use diesel::{prelude::*, query_dsl::RunQueryDsl, Queryable};
use tracing::{debug, error};
use uuid::Uuid;

use crate::database::schema::{cron_job_variables, cron_job_variables::dsl::*};

use super::DbConnection;

#[derive(Debug, Queryable)]
#[diesel(belongs_to(CronJob, foreign_key = cron_job_id))]
#[diesel(table_name = cron_job_variables)]
pub struct CronJobVariable {
    pub id: String,
    pub name: String,
    pub value: String,
    pub cron_job_id: String,
    pub date_created: String,
    pub date_updated: Option<String>,
}

pub struct InsertCronJobVariable<'a> {
    pub id: String,
    pub name: &'a str,
    pub value: &'a str,
    pub cron_job_id: &'a str,
}

impl<'a> InsertCronJobVariable<'a> {
    pub fn new(kv: (&'a String, &'a String), job_id: &'a str) -> Self {
        let cv_id = Uuid::new_v4().to_string();
        let (cv_name, cv_value) = kv;
        Self {
            id: cv_id,
            name: cv_name,
            value: cv_value,
            cron_job_id: job_id,
        }
    }
}

pub fn select_by_cron_job_id(
    conn: &mut DbConnection,
    cv_cron_job_id: &str,
) -> Result<Vec<CronJobVariable>> {
    debug!("loading all variables for cron job with id: {cv_cron_job_id}");
    cron_job_variables
        .filter(cron_job_id.eq(cv_cron_job_id))
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

pub fn insert_many(conn: &mut DbConnection, models: &[InsertCronJobVariable]) -> Result<()> {
    for model in models {
        diesel::insert_into(cron_job_variables::table)
            .values((
                id.eq(&model.id),
                name.eq(model.name),
                value.eq(model.value),
                cron_job_id.eq(model.cron_job_id),
            ))
            .execute(conn)
            .map(|_| {
                debug!("created new cron job environment variable successfully");
            })
            .map_err(|e| {
                error!("couldn't insert cron job environment variable due to {e}");
                anyhow!(e)
            })?;
    }
    Ok(())
}

pub fn delete_by_cron_job_id(conn: &mut DbConnection, cev_cron_job_id: &str) -> Result<()> {
    debug!("deleting cron job variables associated with cron job id: {cev_cron_job_id}");
    diesel::delete(cron_job_variables::table)
        .filter(cron_job_id.eq(cev_cron_job_id))
        .execute(conn)
        .map(|_| {
            debug!("deleted all cron job variables successfully");
        })
        .map_err(|e| {
            error!("couldn't delete cron job variables due to {e}");
            anyhow!(e)
        })
}
