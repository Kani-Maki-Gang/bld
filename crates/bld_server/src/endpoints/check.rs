use std::sync::Arc;

use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Responder, get};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use bld_runner::{Load, Yaml};
use tracing::info;

#[get("/v1/check")]
pub async fn get(
    _user: User,
    config: Data<BldConfig>,
    fs: Data<FileSystem>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /check route");
    match do_check(Arc::clone(&config), Arc::clone(&fs), &params).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_check(
    config: Arc<BldConfig>,
    fs: Arc<FileSystem>,
    params: &PipelineQueryParams,
) -> Result<()> {
    let content = fs.read(&params.pipeline).await?;
    let pipeline = Yaml::load_with_verbose_errors(&content)?;
    pipeline.validate_with_verbose_errors(config, fs).await
}
