use crate::config::BldConfig;
use crate::run::Pipeline;
use crate::server::User;
use crate::persist::pipeline;
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[post("/inspect")]
pub async fn inspect(
    user: Option<User>, 
    config: web::Data<BldConfig>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    body: web::Json<String>
) -> impl Responder {
    info!("Reached handler for /inspect route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match do_inspect(config.get_ref(), pool.get_ref(), &body.into_inner()) {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn do_inspect(config: &BldConfig, pool: &Pool<ConnectionManager<SqliteConnection>>, name: &str) -> anyhow::Result<String> {
    let conn = pool.get()?;
    let pipeline = pipeline::select_by_name(&conn, name)?;
    Pipeline::read_in_server(config, &pipeline.id)
}
