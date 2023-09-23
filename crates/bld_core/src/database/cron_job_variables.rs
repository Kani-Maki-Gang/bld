use anyhow::{anyhow, Result};
use bld_entities::cron_job_variables::{self, Entity as CronJobVariableEntity};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use tracing::{debug, error};
use uuid::Uuid;

pub use bld_entities::cron_job_variables::Model as CronJobVariable;

pub struct InsertCronJobVariable {
    pub id: String,
    pub name: String,
    pub value: String,
    pub cron_job_id: String,
}

impl InsertCronJobVariable {
    pub fn new(kv: (&String, &String), job_id: &str) -> Self {
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

pub async fn select_by_cron_job_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cv_cron_job_id: &str,
) -> Result<Vec<CronJobVariable>> {
    debug!("loading all variables for cron job with id: {cv_cron_job_id}");
    CronJobVariableEntity::find()
        .filter(cron_job_variables::Column::CronJobId.eq(cv_cron_job_id))
        .all(conn)
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

pub async fn insert_many<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    models: &[InsertCronJobVariable],
) -> Result<()> {
    if models.is_empty() {
        return Ok(());
    }

    let models: Vec<cron_job_variables::ActiveModel> = models
        .iter()
        .map(|m| cron_job_variables::ActiveModel {
            id: Set(m.id.to_owned()),
            name: Set(m.name.to_owned()),
            value: Set(m.value.to_owned()),
            cron_job_id: Set(m.cron_job_id.to_owned()),
            ..Default::default()
        })
        .collect();

    CronJobVariableEntity::insert_many(models)
        .exec(conn)
        .await
        .map(|_| {
            debug!("created new cron job environment variable successfully");
        })
        .map_err(|e| {
            error!("couldn't insert cron job environment variable due to {e}");
            anyhow!(e)
        })
}

pub async fn delete_by_cron_job_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cev_cron_job_id: &str,
) -> Result<()> {
    debug!("deleting cron job variables associated with cron job id: {cev_cron_job_id}");
    CronJobVariableEntity::delete_many()
        .filter(cron_job_variables::Column::CronJobId.eq(cev_cron_job_id))
        .exec(conn)
        .await
        .map(|_| {
            debug!("deleted all cron job variables successfully");
        })
        .map_err(|e| {
            error!("couldn't delete cron job variables due to {e}");
            anyhow!(e)
        })
}
