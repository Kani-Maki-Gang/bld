use crate::cron::CronScheduler;
use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use anyhow::Result;
use bld_core::fs::FileSystem;
use bld_dtos::PushInfo;
use bld_runner::{Load, VersionedPipeline, Yaml};
use tracing::{error, info};

#[post("/push")]
pub async fn post(
    _: User,
    fs: Data<FileSystem>,
    cron: Data<CronScheduler>,
    info: Json<PushInfo>,
) -> impl Responder {
    info!("Reached handler for /push route");
    match do_push(&fs, &cron, &info).await {
        Ok(()) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_push(
    fs: &FileSystem,
    cron: &CronScheduler,
    info: &PushInfo,
) -> Result<()> {
    fs.create(&info.name, &info.content, true).await?;
    let pipeline: VersionedPipeline = Yaml::load(&info.content)?;
    let remove_res = match pipeline.cron() {
        Some(schedule) => cron.upsert_default(schedule, &info.name).await,
        None => cron.remove_by_pipeline(&info.name).await,
    };
    remove_res.map_err(|e| {
        error!("{e}");
        e
    })
}
