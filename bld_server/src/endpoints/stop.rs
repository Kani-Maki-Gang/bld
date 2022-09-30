use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use anyhow::Result;
use bld_core::database::pipeline_runs;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[post("/stop")]
pub async fn stop(
    user: Option<User>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    req: Json<String>,
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

fn do_stop(pool: &Pool<ConnectionManager<SqliteConnection>>, id: &str) -> Result<()> {
    let conn = pool.get()?;
    pipeline_runs::update_stopped(&conn, id, true).map(|_| ())
}
