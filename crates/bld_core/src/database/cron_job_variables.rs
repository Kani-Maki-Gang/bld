use anyhow::{anyhow, Result};
use bld_entities::cron_job_variables;
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait};
use tracing::{debug, error};
use uuid::Uuid;

pub use bld_entities::cron_job_variables::Entity as CronJobVariable;

pub struct InsertCronJobVariable {
    pub id: String,
    pub name: String,
    pub value: String,
    pub cron_job_id: String,
}

impl InsertCronJobVariable {
    pub fn new(kv: (&str, &str), job_id: &str) -> Self {
        let cv_id = Uuid::new_v4().to_string();
        let (cv_name, cv_value) = kv;
        Self {
            id: cv_id,
            name: cv_name.to_owned(),
            value: cv_value.to_owned(),
            cron_job_id: job_id.to_owned(),
        }
    }
}

pub async fn select_by_cron_job_id(
    conn: &DatabaseConnection,
    cv_cron_job_id: &str,
) -> Result<Vec<CronJobVariable>> {
    debug!("loading all variables for cron job with id: {cv_cron_job_id}");
    CronJobVariable::find()
        .filter(cron_job_variables::Column::CronJobId.eq(cv_cron_job_id))
        .load(conn)
        .await
        .map(|cev| {
            debug!("loaded cron job variables successfully");
            cev
        })
        .map_err(|e| {
            error!("couldn't load cron job variables due to {e}");
            anyhow!(e)
        })
}

pub async fn insert_many(
    conn: &DatabaseConnection,
    models: &[InsertCronJobVariable],
) -> Result<()> {
    let models: Vec<CronJobVariable> = models
        .iter()
        .map(|m| cron_job_variables::ActiveModel {
            id: Set(m.id),
            name: Set(m.name),
            value: Set(m.value),
            cron_job_id: Set(m.cron_job_id),
            ..Default::default()
        })
        .collect();

    CronJobVariable::insert_many(models)
        .exec(conn)
        .map(|_| {
            debug!("created new cron job environment variable successfully");
        })
        .map_err(|e| {
            error!("couldn't insert cron job environment variable due to {e}");
            anyhow!(e)
        })
}

pub async fn delete_by_cron_job_id(conn: &DatabaseConnection, cev_cron_job_id: &str) -> Result<()> {
    debug!("deleting cron job variables associated with cron job id: {cev_cron_job_id}");
    CronJobVariable::delete_many()
        .filter(cron_job_variables::Column::CronJobId.eq(cev_cron_job_id))
        .exec(conn)
        .map(|_| {
            debug!("deleted all cron job variables successfully");
        })
        .map_err(|e| {
            error!("couldn't delete cron job variables due to {e}");
            anyhow!(e)
        })
}
