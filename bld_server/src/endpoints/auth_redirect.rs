use actix_web::{get, web, HttpResponse, Responder};
use serde_derive::Deserialize;
use tracing::info;

#[derive(Deserialize)]
pub struct AuthRedirectInfo {
    pub code: String,
    pub state: String,
}

#[get("/authRedirect")]
pub async fn auth_redirect(info: web::Query<AuthRedirectInfo>) -> impl Responder {
    info!("Reached handler for /authRedirect route");
    let message = format!("code: {}, state: {}", info.code, info.state);
    HttpResponse::Ok().body(message)
}
