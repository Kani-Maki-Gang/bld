use crate::extractors::User;
use crate::helpers::enqueue_worker;
use crate::requests::RunInfo;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_supervisor::base::ServerMessages;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use tokio::sync::mpsc::Sender;
use tracing::info;

#[post("/run")]
pub async fn run(
    user: Option<User>,
    proxy: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    enqueue_tx: Data<Sender<ServerMessages>>,
    data: Json<RunInfo>,
) -> impl Responder {
    info!("reached handler for /run route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let user = user.unwrap();
    match enqueue_worker(&user, proxy, pool, enqueue_tx, data.into_inner()) {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
