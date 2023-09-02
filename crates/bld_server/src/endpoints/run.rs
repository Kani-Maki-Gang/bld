use std::sync::Arc;

use crate::{
    extractors::User,
    supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker},
};
use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use bld_core::{
    database::DbConnection, messages::ExecClientMessage, proxies::PipelineFileSystemProxy,
};
use diesel::r2d2::{ConnectionManager, Pool};
use tracing::info;

#[post("/run")]
pub async fn post(
    user: User,
    proxy: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<DbConnection>>>,
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
