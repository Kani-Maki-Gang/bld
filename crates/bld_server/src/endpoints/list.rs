use crate::extractors::User;
use actix_web::{get, web::Data, HttpResponse};
use anyhow::Result;
use bld_core::proxies::PipelineFileSystemProxy;
use tracing::info;

#[get("/list")]
pub async fn get(_: User, prx: Data<PipelineFileSystemProxy>) -> HttpResponse {
    info!("Reached handler for /list route");
    match find_pipelines(prx.get_ref()) {
        Ok(pips) => HttpResponse::Ok().json(pips),
        Err(_) => HttpResponse::BadRequest().body("no pipelines found"),
    }
}

fn find_pipelines(prx: &PipelineFileSystemProxy) -> Result<String> {
    Ok(prx.list()?.join("\n"))
}
