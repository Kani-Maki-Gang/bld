use crate::config::definitions::TOOL_DIR;
use crate::helpers::fs::IsYaml;
use crate::path;
use crate::server::User;
use actix_web::{post, web, HttpResponse};
use std::fs::remove_file;
use std::path::PathBuf;
use tracing::{error, info};

#[post("/remove")]
pub async fn remove(user: Option<User>, body: web::Json<String>) -> HttpResponse {
    info!("Reached handler for /remove route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
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
