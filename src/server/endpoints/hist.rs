use crate::persist::pipeline;
use crate::server::User;
use actix_web::{get, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;

#[get("/hist")]
pub async fn hist(
    (user, db_pool): (
        Option<User>,
        web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    ),
) -> impl Responder {
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
    let pipelines = pipeline::select_all(&connection).unwrap_or_else(|_| vec![]);
    let mut info = String::new();
    if !pipelines.is_empty() {
        info = format!(
            "{0: <30} | {1: <36} | {2: <15} | {3: <7} | {4: <19} | {5: <19}",
            "pipeline", "id", "user", "running", "start time", "end time",
        );
        for entry in pipelines.iter() {
            info = format!(
                "{0}\n{1: <30} | {2: <36} | {3: <15} | {4: <7} | {5: <19} | {6: <19}",
                info,
                entry.name,
                entry.id,
                entry.user,
                entry.running,
                entry.start_date_time,
                entry.end_date_time.as_ref().unwrap_or(&String::new())
            );
        }
    }
    Ok(info)
}
