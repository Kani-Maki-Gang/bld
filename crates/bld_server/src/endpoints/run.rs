use std::sync::Arc;

use crate::{
    extractors::User,
    supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker},
};
use actix_web::{
    HttpResponse, Responder, post,
    web::{Data, Json},
};
use bld_core::fs::FileSystem;
use bld_models::dtos::ExecClientMessage;
use sea_orm::DatabaseConnection;
use tracing::info;

#[post("/v1/run")]
pub async fn post(
    user: User,
    fs: Data<FileSystem>,
    conn: Data<DatabaseConnection>,
    supervisor: Data<SupervisorMessageSender>,
    data: Json<ExecClientMessage>,
) -> impl Responder {
    info!("reached handler for /run route");

    let result = enqueue_worker(
        &user.name,
        Arc::clone(&fs),
        Arc::clone(&conn),
        Arc::clone(&supervisor),
        data.into_inner(),
    )
    .await;

    match result {
        Ok(run_id) => HttpResponse::Ok().json(run_id),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
