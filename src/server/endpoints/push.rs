use crate::push::PushInfo;
use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use tracing::info;

#[post("/push")]
pub async fn push(user: Option<User>, info: web::Json<PushInfo>) -> impl Responder {
    info!("Reached handler for /push route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let info = info.into_inner();
    match Pipeline::create(&info.name, &info.content) {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
