use std::sync::Arc;

use crate::extractors::User;
use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use bld_runner::VersionedFile;
use tracing::info;

#[get("/v1/deps")]
pub async fn get(
    _user: User,
    config: Data<BldConfig>,
    fs: Data<FileSystem>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /deps route");
    match do_deps(config, fs, params.into_inner()).await {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_deps(
    config: Data<BldConfig>,
    fs: Data<FileSystem>,
    params: PipelineQueryParams,
) -> Result<Vec<String>> {
    let config = Arc::clone(&config);
    let fs = Arc::clone(&fs);
    let dependencies = VersionedFile::dependencies(config, fs, params.pipeline).await?;
    Ok(dependencies.into_keys().collect())
}
