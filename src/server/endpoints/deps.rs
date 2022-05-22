use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use tracing::info;

#[post("/deps")]
pub async fn deps(user: Option<User>, body: web::Json<String>) -> impl Responder {
    info!("Reached handler for /deps route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match Pipeline::deps(&name).map(|hs| hs.into_iter().map(|(n, _)| n).collect::<Vec<String>>()) {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
