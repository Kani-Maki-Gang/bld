use crate::config::BldConfig;
use crate::server::User;
use crate::persist::Database;
use crate::types::Result;
use actix_web::{get, HttpResponse, Responder};

#[get("/hist")]
pub async fn hist(user: Option<User>) -> impl Responder {
    if let None = user { return HttpResponse::Unauthorized().body(""); }

    match history_info() {
        Ok(ls) => HttpResponse::Ok().body(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn history_info() -> Result<String> {
    let config = BldConfig::load()?;
    let db = Database::connect(&config.local.db)?;
    let pipelines = db.all()?;
    let info = pipelines
        .iter()
        .map(|p| p.to_string())
        .fold(String::new(), |acc, n| format!("{}\n{}\n", acc, n));
    Ok(info)
}
