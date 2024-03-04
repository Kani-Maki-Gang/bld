use crate::extractors::User;
use actix_web::{get, web::Data, HttpResponse};
use anyhow::Result;
use bld_core::fs::FileSystem;
use tracing::info;

#[get("/v1/list")]
pub async fn get(_: User, fs: Data<FileSystem>) -> HttpResponse {
    info!("Reached handler for /list route");
    match find_pipelines(fs.get_ref()).await {
        Ok(pips) => HttpResponse::Ok().json(pips),
        Err(_) => HttpResponse::BadRequest().body("no pipelines found"),
    }
}

async fn find_pipelines(fs: &FileSystem) -> Result<String> {
    Ok(fs.list().await?.join("\n"))
}
