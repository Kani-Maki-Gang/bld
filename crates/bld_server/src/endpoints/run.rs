use std::sync::Arc;

use crate::extractors::User;
use crate::supervisor::channel::SupervisorMessageSender;
use crate::supervisor::helpers::enqueue_worker;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use bld_core::messages::ExecClientMessage;
use bld_core::proxies::PipelineFileSystemProxy;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use tracing::info;

#[post("/run")]
pub async fn run(
    user: User,
    proxy: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    supervisor: Data<SupervisorMessageSender>,
    data: Json<ExecClientMessage>,
) -> impl Responder {
    info!("reached handler for /run route");

    let result = enqueue_worker(
        &user.name,
        Arc::clone(&proxy),
        Arc::clone(&pool),
        Arc::clone(&supervisor),
        data.into_inner(),
    )
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
