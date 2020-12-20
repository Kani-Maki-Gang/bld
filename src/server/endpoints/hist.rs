use crate::config::BldConfig;
use crate::persist::Database;
use crate::types::Result;
use actix_web::{get, HttpResponse, Responder};

#[get("/hist")]
pub async fn hist() -> impl Responder {
    match history_info() {
        Ok(ls) => HttpResponse::Ok().body(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn format(arg1: &str, arg2: &str, arg3: &str) -> String {
    format!("{0: <40} | {1: <30} | {2: <10}", arg1, arg2, arg3)
}

fn history_info() -> Result<String> {
    let config = BldConfig::load()?;
    let db = Database::connect(&config.local.db)?;
    let pipelines = db.all()?;
    let init = format("id", "name", "running");
    let info = pipelines
        .iter()
        .map(|p| format(&p.id, &p.name, &p.running.to_string()))
        .fold(init, |acc, n| format!("{}\n{}", acc, n));
    Ok(info)
}
