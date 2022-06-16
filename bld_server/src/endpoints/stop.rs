use crate::extractors::User;
use actix_web::{post, web, HttpResponse, Responder};
use bld_core::database::pipeline_runs;
use diesel::r2d2::{Pool, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[post("/stop")]
pub async fn stop(
    user: Option<User>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    req: web::Json<String>,
) -> impl Responder {
    info!("Reached handler for /stop route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let id = req.into_inner();
    match do_stop(pool.get_ref(), &id) {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(_) => HttpResponse::BadRequest().body("pipeline not found"),
    }
}

fn do_stop(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    id: &str
) -> anyhow::Result<()> {
    let conn = pool.get()?;
    pipeline_runs::update_stopped(&conn, id, true).map(|_| ())
}
