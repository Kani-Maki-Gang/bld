use crate::persist::PipelineModel;
use crate::server::User;
use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{Pool, ConnectionManager};
use actix_web::{get, web, HttpResponse, Responder};

#[get("/hist")]
pub async fn hist((user, db_pool): (Option<User>, web::Data<Pool<ConnectionManager<SqliteConnection>>>)) -> impl Responder {
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }

    match history_info(db_pool.get_ref()) {
        Ok(ls) => HttpResponse::Ok().body(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn history_info(db_pool: &Pool<ConnectionManager<SqliteConnection>>) -> anyhow::Result<String> {
    let connection = db_pool.get()?;
    let pipelines = PipelineModel::select_all(&connection).unwrap_or_else(|_| vec![]);
    let info = pipelines
        .iter()
        .map(|p| p.to_string())
        .fold(String::new(), |acc, n| format!("{}\n{}\n", acc, n));
    Ok(info)
}
