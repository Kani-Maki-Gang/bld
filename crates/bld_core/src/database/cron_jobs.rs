use anyhow::{anyhow, Result};
use diesel::Insertable;
use diesel::{prelude::*, query_dsl::RunQueryDsl, sqlite::SqliteConnection, Connection, Queryable};
use tracing::{debug, error};

use crate::database::schema::cron_jobs;
use crate::database::schema::cron_jobs::dsl::*;
use crate::database::{cron_job_environment_variables, cron_job_variables};

use super::cron_job_environment_variables::InsertCronJobEnvironmentVariable;
use super::cron_job_variables::InsertCronJobVariable;

#[derive(Debug, Queryable)]
pub struct CronJob {
    pub id: String,
    pub pipeline_id: String,
    pub schedule: String,
}

#[derive(Insertable)]
#[diesel(table_name = cron_jobs)]
pub struct InsertCronJob<'a> {
    pub id: &'a str,
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

pub fn select_by_id(conn: &mut SqliteConnection, cj_id: &str) -> Result<CronJob> {
    debug!("loading last cron job with id: {cj_id}");
    cron_jobs
        .filter(id.eq(cj_id))
        .first(conn)
        .map(|cj| {
            debug!("loaded cron job successfully");
            cj
        })
        .map_err(|e| {
            error!("couldn't load cron job due to {e}");
            anyhow!(e)
        })
}

pub fn select_by_pipeline(conn: &mut SqliteConnection, cj_pipeline_id: &str) -> Result<CronJob> {
    debug!("loading cron job associated with pipeline: {cj_pipeline_id}");
    cron_jobs
        .filter(pipeline_id.eq(cj_pipeline_id))
        .first(conn)
        .map(|cj| {
            debug!("loading cron job successfully");
            cj
        })
        .map_err(|e| {
            error!("couldn't load cron job due to {e}");
            anyhow!(e)
        })
}

pub fn insert(
    conn: &mut SqliteConnection,
    cj_model: &InsertCronJob,
    cv_models: &Option<Vec<InsertCronJobVariable>>,
    cve_models: &Option<Vec<InsertCronJobEnvironmentVariable>>,
) -> Result<CronJob> {
    debug!(
        "inserting new cron job entry with pipeline_id: {} and schedule: {}",
        cj_model.pipeline_id, cj_model.schedule
    );
    conn.transaction(|conn| {
        diesel::insert_into(cron_jobs::table)
            .values(cj_model)
            .execute(conn)
            .map_err(|e| {
                error!("couldn't insert cron job due to {e}");
                anyhow!(e)
            })
            .and_then(|_| {
                if let Some(models) = cv_models {
                    cron_job_variables::insert_many(conn, models)
                } else {
                    Ok(())
                }
            })
            .and_then(|_| {
                if let Some(models) = cve_models {
                    cron_job_environment_variables::insert_many(conn, models)
                } else {
                    Ok(())
                }
            })
            .and_then(|_| {
                debug!("created cron job successfully");
                select_by_id(conn, cj_model.id)
            })
    })
}

pub fn delete_by_cron_job_id(conn: &mut SqliteConnection, cj_id: &str) -> Result<()> {
    debug!("deleting cron job with id: {cj_id}");
    conn.transaction(|conn| {
        cron_job_variables::delete_by_cron_job_id(conn, cj_id)
            .and_then(|_| cron_job_environment_variables::delete_by_cron_job_id(conn, cj_id))
            .and_then(|_| {
                diesel::delete(cron_jobs::table)
                    .filter(id.eq(cj_id))
                    .execute(conn)
                    .map(|_| {
                        debug!("deleted cron job successfully");
                    })
                    .map_err(|e| {
                        error!("couldn't delete cron job due to {e}");
                        anyhow!(e)
                    })
            })
    })
}
