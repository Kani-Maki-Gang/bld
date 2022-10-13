use crate::extractors::User;
use crate::requests::RunInfo;
use actix_web::rt::spawn;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use anyhow::{bail, Result};
use bld_core::database::pipeline_runs;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_supervisor::base::ServerMessages;
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info};
use uuid::Uuid;

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

pub fn enqueue_worker(
    user: &User,
    proxy: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    enqueue_tx: Data<Sender<ServerMessages>>,
    data: RunInfo,
) -> Result<String> {
    let path = proxy.path(&data.name)?;
    debug!(
        "enqueue pipeline with name: {} and path: {:?}",
        data.name, path
    );
    if !path.is_yaml() {
        bail!("pipeline file not found");
    }
    let run_id = Uuid::new_v4().to_string();
    let run_id_clone = run_id.clone();
    let conn = pool.get()?;
    pipeline_runs::insert(&conn, &run_id, &data.name, &user.name)?;
    let variables = data.variables.map(hash_map_to_var_string);
    let environment = data.environment.map(hash_map_to_var_string);

    let enqueue_tx = enqueue_tx.clone();
    spawn(async move {
        let msg = ServerMessages::Enqueue {
            pipeline: data.name.to_string(),
            run_id: run_id_clone,
            variables,
            environment,
        };
        match enqueue_tx.send(msg).await {
            Ok(_) => debug!("sent message to supervisor receiver"),
            Err(e) => error!("unable to send message to supervisor receiver. {e}"),
        }
    });
    Ok(run_id)
}

fn hash_map_to_var_string(hmap: HashMap<String, String>) -> String {
    hmap.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<String>>()
        .join(" ")
}
