use crate::config::definitions::TOOL_DIR;
use crate::helpers::fs::IsYaml;
use crate::server::User;
use crate::path;
use actix_web::{post, web, HttpResponse};
use std::path::PathBuf;
use std::fs::remove_file;
use tracing::{info, error};

#[post("/remove")]
pub async fn remove(user: Option<User>, body: web::Json<String>) -> HttpResponse {
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    info!("Reached handler for /remove route");
    let path = path![TOOL_DIR, body.into_inner()];
    info!("Request to remove file {}", path.display().to_string());
    if path.is_yaml() {
        if let Err(e) = remove_file(path) {
            error!("{e}");
            return HttpResponse::BadRequest().body("unable to remove pipeline");
        }
        return HttpResponse::Ok().body("");
    }
    HttpResponse::BadRequest().body("pipeline not found")
}

