use crate::pull::PullResponse;
use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use tracing::info;

#[post("/pull")]
pub async fn pull(user: Option<User>, body: web::Json<String>) -> impl Responder {
    info!("Reached handler for /pull route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match Pipeline::read(&name) {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&name, &r)),
        Err(_) => HttpResponse::BadRequest().body("Pipeline not found"),
    }
}
