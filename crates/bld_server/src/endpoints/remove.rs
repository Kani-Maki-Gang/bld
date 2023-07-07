use crate::cron::CronScheduler;
use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{delete, HttpResponse};
use anyhow::Result;
use bld_core::proxies::PipelineFileSystemProxy;
use tracing::info;

#[delete("/remove")]
pub async fn remove(
    _: User,
    prx: Data<PipelineFileSystemProxy>,
    cron: Data<CronScheduler>,
    body: Json<String>,
) -> HttpResponse {
    info!("Reached handler for /remove route");
    match do_remove(&prx, &cron, &body).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_remove(prx: &PipelineFileSystemProxy, cron: &CronScheduler, name: &str) -> Result<()> {
    cron.remove_scheduled_jobs(name).await?;
    prx.remove(name)?;
    Ok(())
}
