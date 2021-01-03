use crate::types::AuthRedirectInfo;
use actix_web::{get, web, HttpResponse};

#[get("/authRedirect")]
pub fn auth_redirect(web::Query(info): web::Query<AuthRedirectInfo>) -> HttpResponse {
    let message = format!("code: {}, state: {}", info.code, info.state);
    HttpResponse::Ok().body(message)
}
