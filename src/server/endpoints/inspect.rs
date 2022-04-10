use crate::server::User;
use crate::run::Pipeline;
use actix_web::{post, web, HttpResponse, Responder};
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
    Ok(std::fs::read_to_string(Pipeline::get_path(name)?)?)
}
