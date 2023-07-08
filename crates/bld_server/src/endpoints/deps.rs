use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Responder};
use anyhow::Result;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::requests::PipelineQueryParams;
use bld_runner::VersionedPipeline;
use tracing::info;

#[get("/deps")]
pub async fn get(
    _user: User,
    prx: Data<PipelineFileSystemProxy>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /deps route");
    match do_deps(prx.get_ref(), &params) {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_deps(prx: &PipelineFileSystemProxy, params: &PipelineQueryParams) -> Result<Vec<String>> {
    let dependencies = VersionedPipeline::dependencies(prx, &params.pipeline)?;
    Ok(dependencies.into_keys().collect())
}
