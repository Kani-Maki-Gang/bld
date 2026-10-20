use crate::cron::CronScheduler;
use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{HttpResponse, Responder, post};
use anyhow::Result;
use bld_core::fs::FileSystem;
use bld_models::{
    dtos::{FileType, PushInfo},
    pipeline,
};
use bld_pkg::PackageManager;
use bld_runner::{VersionedFile, VersionedFileLoader, files::v3::RunnerFile};
use sea_orm::DatabaseConnection;
use tracing::{error, info};

#[post("/v1/push")]
pub async fn post(
    _: User,
    conn: Data<DatabaseConnection>,
    fs: Data<FileSystem>,
    package_manager: Data<PackageManager>,
    cron: Data<CronScheduler>,
    info: Json<PushInfo>,
) -> impl Responder {
    info!("Reached handler for /push route");
    match do_push(&conn, &fs, &package_manager, &cron, &info).await {
        Ok(()) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_push(
    conn: &DatabaseConnection,
    fs: &FileSystem,
    package_manager: &PackageManager,
    cron: &CronScheduler,
    info: &PushInfo,
) -> Result<()> {
    fs.create(&info.name, &info.content, true).await?;
    let loader = VersionedFileLoader::new(package_manager, fs, false);
    let metadata = loader.load(&info.name).await?;
    let file_type = match metadata.file {
        VersionedFile::Version3(RunnerFile::ActionFileType(_)) => FileType::Action,
        _ => FileType::Pipeline,
    };
    pipeline::update_file_type(conn, &info.name, file_type).await?;
    let remove_res = match metadata.file.cron() {
        Some(schedule) => cron.upsert_default(schedule, &info.name).await,
        None => cron.remove_by_pipeline(&info.name).await,
    };
    remove_res.map_err(|e| {
        error!("{e}");
        e
    })
}
