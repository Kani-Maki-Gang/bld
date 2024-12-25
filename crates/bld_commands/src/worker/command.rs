use crate::{command::BldCommand, signals::CommandSignals};
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::{spawn, System};
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::{context::Context, fs::FileSystem, logger::Logger};
use bld_http::WebSocket;
use bld_models::{dtos::WorkerMessages, new_connection_pool, pipeline_runs};
use bld_runner::{files::versioned::FileOrPath, PipelineRunnerBuilder};
use bld_sock::WorkerClient;
use bld_utils::{sync::IntoArc, variables::parse_variables};
use chrono::Utc;
use clap::Args;
use futures::{join, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};
use tracing::{debug, error};

#[derive(Args)]
#[command(
    about = "A sub command that creates a worker process for a bld server in order to run a pipeline."
)]
pub struct WorkerCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 'p',
        long = "pipeline",
        required = true,
        help = "The pipeline id in the current bld server instance"
    )]
    pipeline: String,

    #[arg(
        short = 'r',
        long = "run-id",
        required = true,
        help = "The target pipeline run id"
    )]
    run_id: String,

    #[arg(
        short = 'i',
        long = "input",
        help = "Define value for an input variable. Can be used multiple times"
    )]
    inputs: Vec<String>,

    #[arg(
        short = 'e',
        long = "environment",
        help = "Define value for an environment variable. Can be used multiple times"
    )]
    env: Vec<String>,
}

impl BldCommand for WorkerCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let socket_cfg = config.clone();

            let pipeline = self.pipeline.into_arc();
            let run_id = self.run_id.into_arc();
            let inputs = parse_variables(&self.inputs).into_arc();
            let env = parse_variables(&self.env).into_arc();

            let conn = new_connection_pool(config.clone()).await?.into_arc();
            let start_date = Utc::now().naive_utc();
            pipeline_runs::update_start_date(conn.as_ref(), &run_id, &start_date).await?;
            let start_date = start_date.format("%F %X").to_string();
            let fs = FileSystem::Server {
                config: config.clone(),
                conn: conn.clone(),
            }
            .into_arc();

            let (worker_tx, worker_rx) = channel(4096);
            let worker_tx = Some(worker_tx).into_arc();
            let logger = Logger::file(config.clone(), &run_id).await?.into_arc();
            let context = Context::server(config.clone(), conn, &run_id).into_arc();
            let (cmd_signals, signals_rx) = CommandSignals::new()?;

            let socket_handle = spawn(async move {
                if let Err(e) = connect_to_supervisor(socket_cfg, worker_rx).await {
                    error!("{e}");
                }
            });

            let runner_handle = spawn(async move {
                let pipeline = FileOrPath::Path(&pipeline);

                match PipelineRunnerBuilder::default()
                    .run_id(&run_id)
                    .run_start_time(&start_date)
                    .config(config)
                    .fs(fs)
                    .pipeline(pipeline)
                    .logger(logger)
                    .env(env)
                    .inputs(inputs)
                    .context(context)
                    .ipc(worker_tx)
                    .signals(signals_rx)
                    .build()
                    .await
                {
                    Ok(runner) => {
                        if let Err(e) = runner.run().await {
                            error!("error with runner, {e}");
                        }
                    }
                    Err(e) => error!("failed on building the runner, {e}"),
                }

                let _ = cmd_signals.stop().await;
            });

            let _ = join!(socket_handle, runner_handle);

            Ok(())
        })
    }
}

async fn connect_to_supervisor(
    config: Arc<BldConfig>,
    mut worker_rx: Receiver<WorkerMessages>,
) -> Result<()> {
    let url = format!("{}/v1/ws-worker/", config.local.supervisor.base_url_ws());

    debug!("establishing web socket connection on {}", url);

    let (_, framed) = WebSocket::new(&url)?
        .request()
        .connect()
        .await
        .map_err(|e| {
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
