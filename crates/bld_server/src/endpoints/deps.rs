use crate::extractors::User;
use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, requests::PipelineQueryParams};
use bld_runner::VersionedPipeline;
use tracing::info;

#[get("/deps")]
pub async fn get(
    _user: User,
    config: Data<BldConfig>,
    proxy: Data<PipelineFileSystemProxy>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /deps route");
    match do_deps(&config, &proxy, &params) {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_deps(
    config: &BldConfig,
    proxy: &PipelineFileSystemProxy,
    params: &PipelineQueryParams,
) -> Result<Vec<String>> {
    let dependencies = VersionedPipeline::dependencies(config, proxy, &params.pipeline)?;
    Ok(dependencies.into_keys().collect())
}
