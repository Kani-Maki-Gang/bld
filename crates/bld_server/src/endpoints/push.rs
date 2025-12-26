use crate::cron::CronScheduler;
use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{HttpResponse, Responder, post};
use anyhow::Result;
use bld_core::fs::FileSystem;
use bld_models::dtos::PushInfo;
use bld_pkg::PackageManager;
use bld_runner::VersionedFileLoader;
use tracing::{error, info};

#[post("/v1/push")]
pub async fn post(
    _: User,
    fs: Data<FileSystem>,
    package_manager: Data<PackageManager>,
    cron: Data<CronScheduler>,
    info: Json<PushInfo>,
) -> impl Responder {
    info!("Reached handler for /push route");
    match do_push(&fs, &package_manager, &cron, &info).await {
        Ok(()) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_push(
    fs: &FileSystem,
    package_manager: &PackageManager,
    cron: &CronScheduler,
    info: &PushInfo,
) -> Result<()> {
    fs.create(&info.name, &info.content, true).await?;
    let loader = VersionedFileLoader::new(package_manager, fs, false);
    let pipeline = loader.load(&info.content).await?;
    let remove_res = match pipeline.cron() {
        Some(schedule) => cron.upsert_default(schedule, &info.name).await,
        None => cron.remove_by_pipeline(&info.name).await,
    };
    remove_res.map_err(|e| {
        error!("{e}");
        e
    })
}
