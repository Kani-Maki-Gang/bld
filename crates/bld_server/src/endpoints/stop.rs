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
    _: User,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    req: Json<String>,
) -> impl Responder {
    info!("Reached handler for /stop route");
    let id = req.into_inner();
    match do_stop(pool.get_ref(), &id) {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(_) => HttpResponse::BadRequest().body("pipeline not found"),
    }
}

fn do_stop(pool: &Pool<ConnectionManager<SqliteConnection>>, id: &str) -> Result<()> {
    let mut conn = pool.get()?;
    pipeline_runs::update_stopped(&mut conn, id, true).map(|_| ())
}
