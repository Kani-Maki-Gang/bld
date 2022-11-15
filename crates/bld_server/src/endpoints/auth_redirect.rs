use crate::requests::AuthRedirectInfo;
use actix_web::{get, web, HttpResponse, Responder};
use tracing::info;

#[get("/authRedirect")]
pub async fn auth_redirect(info: web::Query<AuthRedirectInfo>) -> impl Responder {
    info!("Reached handler for /authRedirect route");
    let message = format!("code: {}, state: {}", info.code, info.state);
    HttpResponse::Ok().json(message)
}
