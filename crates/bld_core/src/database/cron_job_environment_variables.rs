use anyhow::{anyhow, Result};
use bld_entities::cron_job_environment_variables;
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{debug, error};
use uuid::Uuid;

pub use bld_entities::cron_job_environment_variables::Entity as CronJobEnvironmentVariable;

pub struct InsertCronJobEnvironmentVariable {
    pub id: String,
    pub name: String,
    pub value: String,
    pub cron_job_id: String,
}

impl InsertCronJobEnvironmentVariable {
    pub fn new(kv: (&str, &str), job_id: &str) -> Self {
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

pub async fn select_by_cron_job_id(
    conn: &DatabaseConnection,
    cev_cron_job_id: &str,
) -> Result<Vec<CronJobEnvironmentVariable>> {
    debug!("loading all environment variables for cron job with id: {cev_cron_job_id}");
    CronJobEnvironmentVariable::find()
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

pub async fn insert_many(
    conn: &DatabaseConnection,
    models: &[InsertCronJobEnvironmentVariable],
) -> Result<()> {
    let models: Vec<CronJobEnvironmentVariable> = models
        .iter()
        .map(|m| cron_job_environment_variables::ActiveModel {
            id: Set(m.id),
            name: Set(m.name),
            value: Set(m.value),
            cron_job_id: Set(m.cron_job_id),
            ..Default::default()
        })
        .collect();

    CronJobEnvironmentVariable::insert_many(models)
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

pub async fn delete_by_cron_job_id(conn: &DatabaseConnection, cev_cron_job_id: &str) -> Result<()> {
    debug!(
        "deleting cron job environment variables associated with cron job id: {cev_cron_job_id}"
    );
    CronJobEnvironmentVariable::delete_many()
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
