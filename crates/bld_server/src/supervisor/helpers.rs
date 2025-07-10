use crate::supervisor::channel::SupervisorMessageSender;
use anyhow::{Result, bail};
use bld_core::fs::FileSystem;
use bld_models::{
    dtos::ExecClientMessage,
    pipeline_runs::{self, InsertPipelineRun},
};
use bld_utils::fs::IsYaml;
use sea_orm::DatabaseConnection;
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, error};
use uuid::Uuid;

pub async fn enqueue_worker(
    user_name: &str,
    fs: Arc<FileSystem>,
    conn: Arc<DatabaseConnection>,
    supervisor_sender: Arc<SupervisorMessageSender>,
    data: ExecClientMessage,
) -> Result<String> {
    let ExecClientMessage::EnqueueRun {
        name,
        env: environment,
        inputs: variables,
    } = data;

    let path = fs.path(&name).await?;
    if !path.is_yaml() {
        bail!("pipeline file not found");
    }

    let run_id = Uuid::new_v4().to_string();
    let model = InsertPipelineRun {
        id: run_id.to_owned(),
        name: name.to_owned(),
        app_user: user_name.to_owned(),
    };
    pipeline_runs::insert(conn.as_ref(), model).await?;

    let variables = variables.map(hash_map_to_var_string);
    let environment = environment.map(hash_map_to_var_string);

    supervisor_sender
        .enqueue(name, run_id.to_owned(), variables, environment)
        .await
        .map(|_| {
            debug!("sent message to supervisor receiver");
            run_id
        })
        .map_err(|e| {
            error!("{e}");
            e
        })
}

fn hash_map_to_var_string(hmap: HashMap<String, String>) -> Vec<String> {
    hmap.iter().map(|(k, v)| format!("{k}={v}")).collect()
}
