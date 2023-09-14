use anyhow::{anyhow, Result};
use bld_entities::{
    cron_jobs::{self, Entity as CronJobEntity},
    pipeline::{self, Entity as Pipeline},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, TransactionTrait,
};
use tracing::{debug, error};

use super::{
    cron_job_environment_variables::{self, InsertCronJobEnvironmentVariable},
    cron_job_variables::{self, InsertCronJobVariable},
};

pub use bld_entities::cron_jobs::Model as CronJob;

pub struct InsertCronJob {
    pub id: String,
    pub pipeline_id: String,
    pub schedule: String,
    pub is_default: bool,
}

pub struct UpdateCronJob {
    pub id: String,
    pub schedule: String,
}

pub async fn select_all<C: ConnectionTrait + TransactionTrait>(conn: &C) -> Result<Vec<CronJob>> {
    debug!("loading all cron jobs from the database");
    CronJobEntity::find()
        .all(conn)
        .await
        .map(|c| {
            debug!("loaded all cron jobs successfully");
            c
        })
        .map_err(|e| {
            error!("couln't load cron jobs due to {e}");
            anyhow!(e)
        })
}

pub async fn select_by_id<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cj_id: &str,
) -> Result<CronJob> {
    debug!("loading last cron job with id: {cj_id}");
    let model = CronJobEntity::find_by_id(cj_id)
        .one(conn)
        .await
        .map_err(|e| {
            error!("couldn't load cron job due to {e}");
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load cron job due to not found");
            anyhow!("cron job not found")
        })
        .map(|cj| {
            debug!("loaded cron job successfully");
            cj
        })
}

pub async fn select_default_by_pipeline<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cj_pipeline_id: &str,
) -> Result<CronJob> {
    debug!("loading default cron job associated with pipeline: {cj_pipeline_id}");

    let model = CronJobEntity::find()
        .filter(cron_jobs::Column::PipelineId.eq(cj_pipeline_id))
        .filter(cron_jobs::Column::IsDefault.eq(true))
        .one(conn)
        .await
        .map_err(|e| {
            error!("couldn't load cron job due to {e}");
            anyhow!(e)
        })?;

    model
        .ok_or_else(|| {
            error!("couldn't load cron job due to not found");
            anyhow!("cron job not found")
        })
        .map(|cj| {
            debug!("loading cron job successfully");
            cj
        })
}

pub async fn select_by_pipeline<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cj_pipeline_id: &str,
) -> Result<Vec<CronJob>> {
    debug!("loading cron job associated with pipeline: {cj_pipeline_id}");
    CronJobEntity::find()
        .filter(cron_jobs::Column::PipelineId.eq(cj_pipeline_id))
        .all(conn)
        .await
        .map(|cj| {
            debug!("loading cron job successfully");
            cj
        })
        .map_err(|e| {
            error!("couldn't load cron job due to {e}");
            anyhow!(e)
        })
}

pub async fn select_with_filters<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    flt_id: Option<&str>,
    flt_pipeline: Option<&str>,
    flt_schedule: Option<&str>,
    flt_is_default: Option<bool>,
    flt_limit: Option<u64>,
) -> Result<Vec<CronJob>> {
    debug!("loading cron jobs based on filters");

    let mut find = CronJobEntity::find();

    if let Some(flt_id) = flt_id {
        find = find.filter(cron_jobs::Column::Id.eq(flt_id));
    }

    if let Some(flt_pipeline) = flt_pipeline {
        let pipeline = Pipeline::find()
            .filter(pipeline::Column::Name.eq(flt_pipeline))
            .one(conn)
            .await
            .map_err(|e| {
                error!("couldn't load pipeline with name {flt_pipeline} due to {e}");
                anyhow!(e)
            })?
            .ok_or_else(|| {
                error!("couldn't load pipeline due to not found");
                anyhow!("pipeline not found")
            })
            .map(|model| {
                debug!("loaded pipeline with name {flt_pipeline} successfully");
                model
            })?;
        find = find.filter(cron_jobs::Column::PipelineId.eq(pipeline.id));
    }

    if let Some(flt_schedule) = flt_schedule {
        find = find.filter(cron_jobs::Column::Schedule.eq(flt_schedule));
    }

    if let Some(flt_is_default) = flt_is_default {
        find = find.filter(cron_jobs::Column::IsDefault.eq(flt_is_default));
    }

    find = find.limit(flt_limit);

    find.all(conn)
        .await
        .map(|jobs| {
            debug!("loaded cron jobs successfully!");
            jobs
        })
        .map_err(|e| {
            error!("unable to load cron jobs due to {e}");
            anyhow!(e)
        })
}

pub async fn insert<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cj_model: &InsertCronJob,
    cv_models: &Option<Vec<InsertCronJobVariable>>,
    cve_models: &Option<Vec<InsertCronJobEnvironmentVariable>>,
) -> Result<()> {
    debug!(
        "inserting new cron job entry with pipeline_id: {} and schedule: {}",
        cj_model.pipeline_id, cj_model.schedule
    );
    let txn = conn.begin().await?;

    let model = cron_jobs::ActiveModel {
        id: Set(cj_model.id.to_owned()),
        pipeline_id: Set(cj_model.pipeline_id.to_owned()),
        schedule: Set(cj_model.schedule.to_owned()),
        is_default: Set(cj_model.is_default),
        ..Default::default()
    };

    model.insert(&txn).await.map_err(|e| {
        error!("couldn't insert cron job due to {e}");
        anyhow!(e)
    })?;

    if let Some(cv_models) = cv_models.as_ref() {
        cron_job_variables::insert_many(&txn, cv_models).await?;
    }

    if let Some(cve_models) = cve_models.as_ref() {
        cron_job_environment_variables::insert_many(&txn, cve_models).await?;
    }

    debug!("created cron job successfully");
    Ok(())
}

pub async fn update(
    conn: &DatabaseConnection,
    cj_model: &UpdateCronJob,
    cv_models: &Option<Vec<InsertCronJobVariable>>,
    cve_models: &Option<Vec<InsertCronJobEnvironmentVariable>>,
) -> Result<()> {
    debug!("updating cron job entry with id: {}", cj_model.id);
    let txn = conn.begin().await?;

    let mut model = select_by_id(&txn, &cj_model.id).await?.into_active_model();
    model.schedule = Set(cj_model.schedule.to_owned());
    model.update(&txn).await?;

    cron_job_variables::delete_by_cron_job_id(&txn, &cj_model.id).await?;
    if let Some(cv_models) = cv_models.as_ref() {
        cron_job_variables::insert_many(&txn, cv_models).await?;
    }

    cron_job_environment_variables::delete_by_cron_job_id(&txn, &cj_model.id).await?;
    if let Some(cve_models) = cve_models.as_ref() {
        cron_job_environment_variables::insert_many(&txn, cve_models).await?;
    }

    debug!("updated cron job successfully");
    Ok(())
}

pub async fn delete_by_cron_job_id(conn: &DatabaseConnection, cj_id: &str) -> Result<()> {
    debug!("deleting cron job with id: {cj_id}");
    let txn = conn.begin().await?;

    cron_job_variables::delete_by_cron_job_id(&txn, cj_id).await?;
    cron_job_environment_variables::delete_by_cron_job_id(&txn, cj_id).await?;

    select_by_id(&txn, cj_id)
        .await?
        .into_active_model()
        .delete(&txn)
        .await
        .map(|_| {
            debug!("deleted cron job successfully");
        })
        .map_err(|e| {
            error!("couldn't delete cron job due to {e}");
            anyhow!(e)
        })
}

pub async fn delete_by_pipeline<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    cj_pipeline_id: &str,
) -> Result<()> {
    debug!("deleting cron jobs associated with pipeline id: {cj_pipeline_id}");
    let txn = conn.begin().await?;
    let models = select_by_pipeline(&txn, cj_pipeline_id).await?;
    for model in models {
        cron_job_variables::delete_by_cron_job_id(&txn, &model.id).await?;
        cron_job_environment_variables::delete_by_cron_job_id(&txn, &model.id).await?;
        model
            .into_active_model()
            .delete(&txn)
            .await
            .map(|_| {
                debug!("deleted cron job successfully");
            })
            .map_err(|e| {
                error!("couldn't delete cron job due to {e}");
                anyhow!(e)
            })?;
    }
    Ok(())
}
