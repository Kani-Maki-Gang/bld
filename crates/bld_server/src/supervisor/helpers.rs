use crate::supervisor::channel::SupervisorMessageSender;
use anyhow::{bail, Result};
use bld_core::database::pipeline_runs;
use bld_core::messages::ExecClientMessage;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

pub async fn enqueue_worker(
    user_name: &str,
    proxy: Arc<PipelineFileSystemProxy>,
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    supervisor_sender: Arc<SupervisorMessageSender>,
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
    let run = pipeline_runs::insert(&mut conn, &run_id, &name, user_name)?;

    let variables = variables.map(hash_map_to_var_string);
    let environment = environment.map(hash_map_to_var_string);

    supervisor_sender
        .enqueue(name.to_string(), run_id, variables, environment)
        .await
        .map(|_| {
            debug!("sent message to supervisor receiver");
            run.id
        })
        .map_err(|e| {
            error!("{e}");
            e
        })
}

fn hash_map_to_var_string(hmap: HashMap<String, String>) -> Vec<String> {
    hmap.iter().map(|(k, v)| format!("{k}={v}")).collect()
}
