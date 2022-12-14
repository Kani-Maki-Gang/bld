use crate::{extractors::User, requests::CheckQueryParams};
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Responder};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::{Load, Yaml};
use std::sync::Arc;
use tracing::info;

#[get("/check")]
pub async fn check(
    _user: User,
    config: Data<BldConfig>,
    proxy: Data<PipelineFileSystemProxy>,
    params: Query<CheckQueryParams>,
) -> impl Responder {
    info!("Reached handler for /check route");
    match do_check(Arc::clone(&config), Arc::clone(&proxy), params.into_inner()) {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_check(
    config: Arc<BldConfig>,
    proxy: Arc<PipelineFileSystemProxy>,
    params: CheckQueryParams,
) -> Result<()> {
    let content = proxy.read(&params.pipeline)?;
    let pipeline = Yaml::load_with_verbose_errors(&content)?;
    pipeline.validate_with_verbose_errors(config, proxy)
}
