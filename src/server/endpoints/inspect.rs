use crate::{run::Pipeline, helpers::fs::IsYaml};
use crate::server::User;
use anyhow::anyhow;
use actix_web::{post, web, HttpResponse, Responder};
use std::fs::read_to_string;
use tracing::{debug, info};

#[post("/inspect")]
pub async fn inspect(user: Option<User>, body: web::Json<String>) -> impl Responder {
    info!("Reached handler for /inspect route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }

    let name = body.into_inner();
    if name == "config.yaml" {
        debug!("inspect request was for config.yaml file. returing 404");
        return HttpResponse::NotFound().body("");
    }

    match inspect_pipeline(&name) {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn inspect_pipeline(name: &str) -> anyhow::Result<String> {
    let path = Pipeline::get_path(name)?;
    if !path.is_yaml() {
        return Err(anyhow!("Pipeline not found"));
    }
    Ok(read_to_string(path)?)
}
