use crate::cron::CronScheduler;
use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, delete};
use anyhow::Result;
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use tracing::info;

#[delete("/v1/remove")]
pub async fn delete(
    _: User,
    fs: Data<FileSystem>,
    cron: Data<CronScheduler>,
    params: Query<PipelineQueryParams>,
) -> HttpResponse {
    info!("Reached handler for /remove route");
    match do_remove(&fs, &cron, &params).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_remove(
    fs: &FileSystem,
    cron: &CronScheduler,
    params: &PipelineQueryParams,
) -> Result<()> {
    cron.remove_scheduled_jobs(&params.pipeline).await?;
    fs.remove(&params.pipeline).await?;
    Ok(())
}
