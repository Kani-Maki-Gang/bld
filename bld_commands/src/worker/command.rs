use crate::run::parse_variables;
use crate::worker::WorkerClient;
use crate::BldCommand;
use actix::{io::SinkWrite, Actor, StreamHandler};
use anyhow::anyhow;
use awc::Client;
use bld_config::{path, BldConfig};
use bld_core::database::{new_connection_pool, pipeline_runs};
use bld_core::execution::PipelineExecution;
use bld_core::logger::FileLogger;
use bld_core::proxies::ServerPipelineProxy;
use bld_runner::RunnerBuilder;
use bld_supervisor::base::WorkerMessages;
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::env::temp_dir;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tracing::{debug, error};

const WORKER: &str = "worker";
const PIPELINE: &str = "pipeline";
const RUN_ID: &str = "run-id";
const VARIABLES: &str = "variables";
const ENVIRONMENT: &str = "environment";

pub struct WorkerCommand;

impl WorkerCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for WorkerCommand {
    fn id(&self) -> &'static str {
        WORKER
    }

    fn interface(&self) -> App<'static, 'static> {
        let pipeline = Arg::with_name(PIPELINE)
            .short("p")
            .long(PIPELINE)
            .help("The pipeline id in the current bld server instance")
            .takes_value(true)
            .required(true);
        let run_id = Arg::with_name(RUN_ID)
            .short("r")
            .long(RUN_ID)
            .help("The target pipeline run id")
            .takes_value(true)
            .required(true);
        let variables = Arg::with_name(VARIABLES)
            .short("v")
            .long(VARIABLES)
            .help("Define values for variables in the server pipeline")
            .multiple(true)
            .takes_value(true);
        let environment = Arg::with_name(ENVIRONMENT)
            .short("e")
            .long(ENVIRONMENT)
            .help("Define values for environment variables in the server pipeline")
            .multiple(true)
            .takes_value(true);
        SubCommand::with_name(WORKER)
            .about("A sub command that creates a worker process for a bld server in order to run a pipeline.")
            .args(&[pipeline, run_id, variables, environment])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let config = BldConfig::load()?;
        let pipeline = matches.value_of(PIPELINE).unwrap_or_default().to_string();
        let run_id = matches.value_of(RUN_ID).unwrap_or_default().to_string();
        let variables = Arc::new(parse_variables(matches, VARIABLES));
        let environment = Arc::new(parse_variables(matches, ENVIRONMENT));
        let (worker_tx, worker_rx) = channel::<WorkerMessages>();
        let socket_path = path![temp_dir(), &config.local.unix_sock]
            .display()
            .to_string();
        let rt = Runtime::new()?;
        rt.block_on(async move {
            let _ = tokio::join!(
                start_runner(config, pipeline, run_id, variables, environment, worker_tx),
                connect_to_supervisor(socket_path, worker_rx),
            );
        });
        Ok(())
    }
}

async fn start_runner(
    config: BldConfig,
    pipeline: String,
    run_id: String,
    variables: Arc<HashMap<String, String>>,
    environment: Arc<HashMap<String, String>>,
    worker_tx: Sender<WorkerMessages>,
) -> anyhow::Result<()> {
    let cfg = Arc::new(config);
    let pool = Arc::new(new_connection_pool(&cfg.local.db)?);
    let conn = pool.get()?;
    let pipeline_run = pipeline_runs::select_by_id(&conn, &run_id)?;
    let start_date_time = pipeline_run.start_date_time;
    let proxy = Arc::new(ServerPipelineProxy::new(cfg.clone(), pool.clone()));
    let logger = Arc::new(Mutex::new(FileLogger::new(cfg.clone(), &run_id)?));
    let exec = Arc::new(Mutex::new(PipelineExecution::new(pool, &run_id)?));
    let runner = RunnerBuilder::default()
        .run_id(&run_id)
        .run_start_time(&start_date_time)
        .config(cfg.clone())
        .proxy(proxy)
        .pipeline(&pipeline)
        .execution(exec)
        .logger(logger)
        .environment(environment)
        .variables(variables)
        .ipc_sender(Arc::new(Some(worker_tx)))
        .build()
        .await?;
    runner.run().await.await
}

async fn connect_to_supervisor(
    _socket_path: String,
    worker_rx: Receiver<WorkerMessages>,
) -> anyhow::Result<()> {
    // let url = format!("unix://{socket_path}/ws-worker/");
    let url = format!("http://127.0.0.1:7000/ws-worker/");
    debug!("establishing web socket connection on {}", url);
    let (_, framed) = Client::new().ws(url).connect().await.map_err(|e| {
        error!("{e}");
        anyhow!(e.to_string())
    })?;
    let (sink, stream) = framed.split();
    let addr = WorkerClient::create(|ctx| {
        WorkerClient::add_stream(stream, ctx);
        WorkerClient::new(SinkWrite::new(sink, ctx))
    });
    addr.send(WorkerMessages::Ack).await?;
    while let Ok(msg) = worker_rx.recv() {
        addr.send(msg).await?;
    }
    Ok(())
}
