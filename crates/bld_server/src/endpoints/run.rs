use crate::SupervisorMessageSender;
use crate::extractors::User;
use crate::helpers::enqueue_worker;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_sock::messages::RunInfo;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use tracing::info;

#[post("/run")]
pub async fn run(
    user: User,
    proxy: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    supervisor_sender: Data<SupervisorMessageSender>,
    data: Json<RunInfo>,
) -> impl Responder {
    info!("reached handler for /run route");
    match enqueue_worker(&user, proxy, pool, supervisor_sender, data.into_inner()) {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
