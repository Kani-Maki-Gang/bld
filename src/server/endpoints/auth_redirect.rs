use actix_web::{get, web, HttpResponse};
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct AuthRedirectInfo {
    pub code: String,
    pub state: String,
}

#[get("/authRedirect")]
pub fn auth_redirect(web::Query(info): web::Query<AuthRedirectInfo>) -> HttpResponse {
    let message = format!("code: {}, state: {}", info.code, info.state);
    HttpResponse::Ok().body(message)
}
