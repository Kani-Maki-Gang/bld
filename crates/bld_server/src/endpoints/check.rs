use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Responder, get};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use bld_pkg::PackageManager;
use bld_runner::{VersionedFile, VersionedFileLoader};
use tracing::info;

#[get("/v1/check")]
pub async fn get(
    _user: User,
    config: Data<BldConfig>,
    fs: Data<FileSystem>,
    package_manager: Data<PackageManager>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /check route");
    match do_check(&config, &fs, &package_manager, &params).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_check(
    config: &BldConfig,
    fs: &FileSystem,
    package_manager: &PackageManager,
    params: &PipelineQueryParams,
) -> Result<()> {
    let loader = VersionedFileLoader::new(package_manager, fs, true);
    loader.load(&params.pipeline).await?;
    VersionedFile::check_dependency_cycles(config, fs, package_manager, &params.pipeline).await
}
