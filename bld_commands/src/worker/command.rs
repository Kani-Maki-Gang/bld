use crate::{run::parse_variables, BldCommand};
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::{spawn, System};
use anyhow::anyhow;
use awc::Client;
use bld_config::BldConfig;
use bld_core::{
    context::Context,
    database::{new_connection_pool, pipeline_runs},
    execution::Execution,
    logger::Logger,
    proxies::PipelineFileSystemProxy,
};
use bld_runner::RunnerBuilder;
use bld_supervisor::{base::WorkerMessages, sockets::WorkerClient};
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::{join, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};
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

    fn interface(&self) -> App<'static> {
        let pipeline = Arg::with_name(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("The pipeline id in the current bld server instance")
            .takes_value(true)
            .required(true);
        let run_id = Arg::with_name(RUN_ID)
            .short('r')
            .long(RUN_ID)
            .help("The target pipeline run id")
            .takes_value(true)
            .required(true);
        let variables = Arg::with_name(VARIABLES)
            .short('v')
            .long(VARIABLES)
            .help("Define values for variables in the server pipeline")
            .multiple(true)
            .takes_value(true);
        let environment = Arg::with_name(ENVIRONMENT)
            .short('e')
            .long(ENVIRONMENT)
            .help("Define values for environment variables in the server pipeline")
            .multiple(true)
            .takes_value(true);
        SubCommand::with_name(WORKER)
            .about("A sub command that creates a worker process for a bld server in order to run a pipeline.")
            .args(&[pipeline, run_id, variables, environment])
    }

    fn exec(&self, matches: &ArgMatches) -> anyhow::Result<()> {
        let cfg = Arc::new(BldConfig::load()?);
        let socket_cfg = Arc::clone(&cfg);
        let pipeline = Arc::new(matches.value_of(PIPELINE).unwrap_or_default().to_string());
        let run_id = Arc::new(matches.value_of(RUN_ID).unwrap_or_default().to_string());
        let variables = Arc::new(parse_variables(matches, VARIABLES));
        let environment = Arc::new(parse_variables(matches, ENVIRONMENT));
        let pool = Arc::new(new_connection_pool(&cfg.local.db)?);
        let conn = pool.get()?;
        let pipeline_run = pipeline_runs::select_by_id(&conn, &run_id)?;
        let start_date_time = pipeline_run.start_date_time;
        let proxy = Arc::new(PipelineFileSystemProxy::Server {
            config: cfg.clone(),
            pool: pool.clone(),
        });
        let logger = Logger::file_atom(cfg.clone(), &run_id)?;
        let exec = Execution::pipeline_atom(pool.clone(), &run_id);
        let context = Context::containers_atom(pool.clone(), &run_id);
        let (worker_tx, worker_rx) = channel(4096);
        let worker_tx = Arc::new(Some(worker_tx));
        System::new().block_on(async move {
            let socket_handle = spawn(async move {
                if let Err(e) = connect_to_supervisor(socket_cfg, worker_rx).await {
                    error!("{e}");
                }
            });
            let runner_handle = spawn(async move {
                match RunnerBuilder::default()
                    .run_id(&run_id)
                    .run_start_time(&start_date_time)
                    .config(cfg)
                    .proxy(proxy)
                    .pipeline(&pipeline)
                    .execution(exec)
                    .logger(logger)
                    .environment(environment)
                    .variables(variables)
                    .context(context)
                    .ipc(worker_tx)
                    .build()
                    .await
                {
                    Ok(runner) => {
                        let _ = runner.run().await.await.map_err(|e| {
                            error!("{e}");
                            e
                        });
                    }
                    Err(e) => error!("failed on building the runner, {e}"),
                }
            });
            let _ = join!(socket_handle, runner_handle);
        });
        Ok(())
    }
}

async fn connect_to_supervisor(
    config: Arc<BldConfig>,
    mut worker_rx: Receiver<WorkerMessages>,
) -> anyhow::Result<()> {
    let url = format!(
        "ws://{}:{}/ws-worker/",
        config.local.supervisor.host, config.local.supervisor.port
    );
    debug!("establishing web socket connection on {}", url);
    let client = Client::new().ws(url).connect();
    let (_, framed) = client.await.map_err(|e| {
        error!("{e}");
        anyhow!(e.to_string())
    })?;
    let (sink, stream) = framed.split();
    let addr = WorkerClient::create(|ctx| {
        WorkerClient::add_stream(stream, ctx);
        WorkerClient::new(SinkWrite::new(sink, ctx))
    });
    addr.send(WorkerMessages::Ack).await?;
    addr.send(WorkerMessages::WhoAmI {
        pid: std::process::id(),
    })
    .await?;
    while let Some(msg) = worker_rx.recv().await {
        debug!("sending message to supervisor {:?}", msg);
        addr.send(msg).await?
    }
    Ok(())
}
