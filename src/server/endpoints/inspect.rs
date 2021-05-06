use crate::config::definitions;
use crate::path;
use crate::server::User;
use crate::types::Result;
use actix_web::{get, web, HttpResponse, Responder};
use std::path::PathBuf;

#[get("/inspect/{id}")]
pub async fn inspect(user: Option<User>, path: web::Path<(String,)>) -> impl Responder {
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }

    let name = path.into_inner().0;
    if name == "config" {
        return HttpResponse::NotFound().body("");
    }

    match inspect_pipeline(&name) {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn inspect_pipeline(name: &str) -> Result<String> {
    let path = path![
        std::env::current_dir()?,
        definitions::TOOL_DIR,
        format!("{}.yaml", name)
    ];
    Ok(std::fs::read_to_string(path)?)
}
