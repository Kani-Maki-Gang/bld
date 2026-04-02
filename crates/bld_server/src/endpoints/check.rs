use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Responder, get};
use anyhow::Result;
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use bld_pkg::PackageManager;
use bld_runner::VersionedFileLoader;
use tracing::info;

#[get("/v1/check")]
pub async fn get(
    _user: User,
    fs: Data<FileSystem>,
    package_manager: Data<PackageManager>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /check route");
    match do_check(&fs, &package_manager, &params).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_check(
    fs: &FileSystem,
    package_manager: &PackageManager,
    params: &PipelineQueryParams,
) -> Result<()> {
    let loader = VersionedFileLoader::new(package_manager, fs, true);
    loader.load(&params.pipeline).await?;
    Ok(())
}
