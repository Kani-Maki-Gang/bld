use crate::cron::CronScheduler;
use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{delete, HttpResponse};
use anyhow::Result;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::requests::PipelineQueryParams;
use tracing::info;

#[delete("/remove")]
pub async fn delete(
    _: User,
    prx: Data<PipelineFileSystemProxy>,
    cron: Data<CronScheduler>,
    params: Query<PipelineQueryParams>,
) -> HttpResponse {
    info!("Reached handler for /remove route");
    match do_remove(&prx, &cron, &params).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_remove(
    prx: &PipelineFileSystemProxy,
    cron: &CronScheduler,
    params: &PipelineQueryParams,
) -> Result<()> {
    cron.remove_scheduled_jobs(&params.pipeline).await?;
    prx.remove(&params.pipeline).await?;
    Ok(())
}
