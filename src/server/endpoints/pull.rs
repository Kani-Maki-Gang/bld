use crate::config::BldConfig;
use crate::persist::pipeline;
use crate::pull::PullResponse;
use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[post("/pull")]
pub async fn pull(
    user: Option<User>, 
    config: web::Data<BldConfig>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    body: web::Json<String>
) -> impl Responder {
    info!("Reached handler for /pull route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match do_pull(config.get_ref(), pool.get_ref(), &name) {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&name, &r)),
        Err(_) => HttpResponse::BadRequest().body("Pipeline not found"),
    }
}

fn do_pull(
    config: &BldConfig,
    pool: &Pool<ConnectionManager<SqliteConnection>>, 
    name: &str
) -> anyhow::Result<String> {
    let conn = pool.get()?;
    let pip = pipeline::select_by_name(&conn, name)?;
    Pipeline::read_in_server(&config, &pip.id)
}
