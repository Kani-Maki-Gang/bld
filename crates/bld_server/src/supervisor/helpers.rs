use crate::extractors::User;
use crate::supervisor::channel::SupervisorMessageSender;
use actix_web::rt::spawn;
use actix_web::web::Data;
use anyhow::{bail, Result};
use bld_core::database::pipeline_runs;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_sock::messages::ExecClientMessage;
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use std::collections::HashMap;
use tracing::{debug, error};
use uuid::Uuid;

pub fn enqueue_worker(
    user: &User,
    proxy: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    supervisor_sender: Data<SupervisorMessageSender>,
    data: ExecClientMessage,
) -> Result<String> {
    let ExecClientMessage::EnqueueRun {
        name,
        environment,
        variables,
    } = data;

    let path = proxy.path(&name)?;
    if !path.is_yaml() {
        bail!("pipeline file not found");
    }

    let run_id = Uuid::new_v4().to_string();
    let mut conn = pool.get()?;
    let run = pipeline_runs::insert(&mut conn, &run_id, &name, &user.name)?;

    let variables = variables.map(hash_map_to_var_string);
    let environment = environment.map(hash_map_to_var_string);

    spawn(async move {
        let result = supervisor_sender
            .enqueue(name.to_string(), run_id, variables, environment)
            .await;

        match result {
            Ok(_) => debug!("sent message to supervisor receiver"),
            Err(e) => error!("unable to send message to supervisor receiver. {e}"),
        }
    });

    Ok(run.id)
}

fn hash_map_to_var_string(hmap: HashMap<String, String>) -> Vec<String> {
    hmap.iter().map(|(k, v)| format!("{k}={v}")).collect()
}
