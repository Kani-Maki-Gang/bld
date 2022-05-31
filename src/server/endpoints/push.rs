use crate::config::BldConfig;
use crate::persist::pipeline;
use crate::push::PushInfo;
use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use uuid::Uuid;
use tracing::info;

#[post("/push")]
pub async fn push(
    user: Option<User>, 
    config: web::Data<BldConfig>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    info: web::Json<PushInfo>
) -> impl Responder {
    info!("Reached handler for /push route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match do_push(config.get_ref(), pool.get_ref(), &info.into_inner()) {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_push(
    config: &BldConfig,
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    info: &PushInfo
) -> anyhow::Result<()> {
   let conn = pool.get()?; 
   let pip = match pipeline::select_by_name(&conn, &info.name) {
        Ok(p) => p,
        Err(_) => {
            let id = Uuid::new_v4().to_string();
            pipeline::insert(&conn, &id, &info.name)?
        }
   };
   Pipeline::create_in_server(config, &pip.id, &info.content)
}
