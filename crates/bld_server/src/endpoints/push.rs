use crate::cron::CronScheduler;
use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use anyhow::Result;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::requests::PushInfo;
use bld_runner::{Load, VersionedPipeline, Yaml};
use tracing::{error, info};

#[post("/push")]
pub async fn push(
    _: User,
    prx: Data<PipelineFileSystemProxy>,
    cron: Data<CronScheduler>,
    info: Json<PushInfo>,
) -> impl Responder {
    info!("Reached handler for /push route");
    match do_push(&prx, &cron, &info).await {
        Ok(()) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

pub async fn do_push(
    prx: &PipelineFileSystemProxy,
    cron: &CronScheduler,
    info: &PushInfo,
) -> Result<()> {
    prx.create(&info.name, &info.content, true)?;
    let pipeline: VersionedPipeline = Yaml::load(&info.content)?;
    if let Some(schedule) = pipeline.cron() {
        cron.upsert_default(schedule, &info.name)
            .await
            .map_err(|e| {
                error!("{e}");
                e
            })?;
    }
    Ok(())
}
