use anyhow::{anyhow, Result};
use bld_entities::cron_job_environment_variables::{
    self, Entity as CronJobEnvironmentVariableEntity,
};
use chrono::Utc;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use tracing::{debug, error};
use uuid::Uuid;

pub use bld_entities::cron_job_environment_variables::Model as CronJobEnvironmentVariable;

pub struct InsertCronJobEnvironmentVariable {
    pub id: String,
    pub name: String,
    pub value: String,
    pub cron_job_id: String,
}

impl InsertCronJobEnvironmentVariable {
    pub fn new(kv: (&String, &String), job_id: &str) -> Self {
        let cve_id = Uuid::new_v4().to_string();
        let (cve_name, cve_value) = kv;
        Self {
            id: cve_id,
            name: cve_name.to_owned(),
            value: cve_value.to_owned(),
            cron_job_id: job_id.to_owned(),
        }
    }
}

pub async fn select_by_cron_job_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cev_cron_job_id: &str,
) -> Result<Vec<CronJobEnvironmentVariable>> {
    debug!("loading all environment variables for cron job with id: {cev_cron_job_id}");
    CronJobEnvironmentVariableEntity::find()
        .filter(cron_job_environment_variables::Column::CronJobId.eq(cev_cron_job_id))
        .all(conn)
        .await
        .map(|cev| {
            debug!("loaded cron job environment variables successfully");
            cev
        })
        .map_err(|e| {
            error!("couldn't load cron job environment variables due to {e}");
            anyhow!(e)
        })
}

pub async fn insert_many<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    models: &[InsertCronJobEnvironmentVariable],
) -> Result<()> {
    if models.is_empty() {
        return Ok(());
    }

    let models: Vec<cron_job_environment_variables::ActiveModel> = models
        .iter()
        .map(|m| cron_job_environment_variables::ActiveModel {
            id: Set(m.id.to_owned()),
            name: Set(m.name.to_owned()),
            value: Set(m.value.to_owned()),
            cron_job_id: Set(m.cron_job_id.to_owned()),
            date_created: Set(Utc::now().naive_utc()),
            ..Default::default()
        })
        .collect();

    CronJobEnvironmentVariableEntity::insert_many(models)
        .exec(conn)
        .await
        .map(|_| {
            debug!("created new cron job environment variables successfully");
        })
        .map_err(|e| {
            error!("couldn't insert cron job environment variables due to {e}");
            anyhow!(e)
        })
}

pub async fn delete_by_cron_job_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cev_cron_job_id: &str,
) -> Result<()> {
    debug!(
        "deleting cron job environment variables associated with cron job id: {cev_cron_job_id}"
    );
    CronJobEnvironmentVariableEntity::delete_many()
        .filter(cron_job_environment_variables::Column::CronJobId.eq(cev_cron_job_id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("deleted all cron job environment variables successfully");
        })
        .map_err(|e| {
            error!("couldn't delete cron job environment variables due to {e}");
            anyhow!(e)
        })
}
