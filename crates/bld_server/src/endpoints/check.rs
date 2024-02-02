use std::sync::Arc;

use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Responder};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_dtos::PipelineQueryParams;
use bld_runner::{Load, Yaml};
use tracing::info;

#[get("/check")]
pub async fn get(
    _user: User,
    config: Data<BldConfig>,
    proxy: Data<PipelineFileSystemProxy>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /check route");
    match do_check(Arc::clone(&config), Arc::clone(&proxy), &params).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_check(
    config: Arc<BldConfig>,
    proxy: Arc<PipelineFileSystemProxy>,
    params: &PipelineQueryParams,
) -> Result<()> {
    let content = proxy.read(&params.pipeline).await?;
    let pipeline = Yaml::load_with_verbose_errors(&content)?;
    pipeline.validate_with_verbose_errors(config, proxy).await
}
