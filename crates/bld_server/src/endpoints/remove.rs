use crate::cron::{CronScheduler, RemoveJob};
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
    match do_remove(prx.get_ref(), cron.get_ref(), &body.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_remove(prx: &PipelineFileSystemProxy, cron: &CronScheduler, name: &str) -> Result<()> {
    let remove_job = RemoveJob::new(name.to_owned());
    cron.remove(&remove_job).await?;
    prx.remove(name)?;
    Ok(())
}
