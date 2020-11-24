use crate::config::BldConfig;
use crate::persist::Database;
use crate::types::Result;
use actix_web::{get, Responder, HttpResponse};

#[get("/list")]
pub async fn list() -> impl Responder {
    match list_pipelines() {
        Ok(ls) => HttpResponse::Ok().body(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn format(arg1: &str, arg2: &str, arg3: &str) -> String {
    format!("{0: <40} | {1: <30} | {2: <10}", arg1, arg2, arg3)
}

pub fn list_pipelines() -> Result<String> {
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
