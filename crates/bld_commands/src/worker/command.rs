use crate::run::parse_variables;
use crate::BldCommand;
use actix::io::SinkWrite;
use actix::{Actor, StreamHandler};
use actix_web::rt::{spawn, System};
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::context::ContextSender;
use bld_core::database::{new_connection_pool, pipeline_runs};
use bld_core::execution::Execution;
use bld_core::logger::LoggerSender;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::RunnerBuilder;
use bld_sock::clients::WorkerClient;
use bld_sock::messages::WorkerMessages;
use bld_utils::tls::awc_client;
use clap::{Arg, ArgAction, ArgMatches, Command};
use futures::join;
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};
use tracing::{debug, error};

const WORKER: &str = "worker";
const PIPELINE: &str = "pipeline";
const RUN_ID: &str = "run-id";
const VARIABLE: &str = "variable";
const ENVIRONMENT: &str = "environment";

pub struct WorkerCommand;

impl BldCommand for WorkerCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        WORKER
    }

    fn interface(&self) -> Command {
        let pipeline = Arg::new(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("The pipeline id in the current bld server instance")
            .required(true)
            .action(ArgAction::Set);

        let run_id = Arg::new(RUN_ID)
            .short('r')
            .long(RUN_ID)
            .help("The target pipeline run id")
            .action(ArgAction::Set)
            .required(true);

        let variable = Arg::new(VARIABLE)
            .short('v')
            .long(VARIABLE)
            .help("Define value for a variable in the server pipeline")
            .action(ArgAction::Append);

        let environment = Arg::new(ENVIRONMENT)
            .short('e')
            .long(ENVIRONMENT)
            .help("Define values for environment variables in the server pipeline")
            .action(ArgAction::Append);

        Command::new(WORKER)
            .about("A sub command that creates a worker process for a bld server in order to run a pipeline.")
            .args(&[pipeline, run_id, variable, environment])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let cfg = Arc::new(BldConfig::load()?);
        let socket_cfg = Arc::clone(&cfg);

        let pipeline = Arc::new(matches.get_one::<String>(PIPELINE).cloned().unwrap());
        let run_id = Arc::new(matches.get_one::<String>(RUN_ID).cloned().unwrap());
        let variables = Arc::new(parse_variables(matches, VARIABLE));
        let environment = Arc::new(parse_variables(matches, ENVIRONMENT));

        let pool = Arc::new(new_connection_pool(&cfg.local.db)?);
        let mut conn = pool.get()?;
        let pipeline_run = pipeline_runs::select_by_id(&mut conn, &run_id)?;
        let start_date_time = pipeline_run.start_date_time;
        let proxy = Arc::new(PipelineFileSystemProxy::Server {
            config: cfg.clone(),
            pool: pool.clone(),
        });

        let exec = Execution::pipeline_atom(pool.clone(), &run_id);

        let (worker_tx, worker_rx) = channel(4096);
        let worker_tx = Arc::new(Some(worker_tx));

        System::new().block_on(async move {
            let logger = LoggerSender::file_atom(cfg.clone(), &run_id)?;
            let context = ContextSender::containers_atom(pool, &run_id);

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
                        if let Err(e) = runner.run().await.await {
                            error!("error with runner, {e}");
                        }
                    }
                    Err(e) => error!("failed on building the runner, {e}"),
                }
            });

            match join!(socket_handle, runner_handle) {
                (Err(e), Ok(())) | (Ok(()), Err(e)) => error!("{e}"),
                (Err(e1), Err(e2)) => {
                    error!("{e1}");
                    error!("{e2}");
                }
                _ => {}
            }

            Ok(())
        })
    }
}

async fn connect_to_supervisor(
    config: Arc<BldConfig>,
    mut worker_rx: Receiver<WorkerMessages>,
) -> Result<()> {
    let protocol = config.local.supervisor.ws_protocol();
    let url = format!(
        "{protocol}://{}:{}/ws-worker/",
        config.local.supervisor.host, config.local.supervisor.port
    );

    debug!("establishing web socket connection on {}", url);

    let client = awc_client()?.ws(url).connect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_worker_pipeline_arg_accepts_value() {
        let pipeline_name = "mock_pipeline_name";
        let command = WorkerCommand::boxed().interface();
        let matches =
            command.get_matches_from(&["worker", "-r", "mock_run_id", "-p", pipeline_name]);

        assert_eq!(
            matches.get_one::<String>(PIPELINE),
            Some(&pipeline_name.to_string())
        )
    }

    #[test]
    fn cli_worker_run_id_arg_accepts_value() {
        let run_id = "mock_run_id";
        let command = WorkerCommand::boxed().interface();
        let matches =
            command.get_matches_from(&["worker", "-p", "mock_pipeline_name", "-r", run_id]);

        assert_eq!(matches.get_one::<String>(RUN_ID), Some(&run_id.to_string()))
    }

    #[test]
    fn cli_worker_variables_arg_accepts_multiple_values() {
        let variable_name = "mock_variable";
        let command = WorkerCommand::boxed().interface();
        let matches = command.get_matches_from(&[
            "worker",
            "-p",
            "mock_pipeline_name",
            "-r",
            "mock_run_id",
            "-v",
            variable_name,
            "-v",
            variable_name,
            "-v",
            variable_name,
        ]);

        assert_eq!(
            matches.get_many::<String>(VARIABLE).map(|v| v.len()),
            Some(3)
        )
    }

    #[test]
    fn cli_worker_environment_variables_arg_accepts_multiple_values() {
        let environment_variable_name = "mock_environment_variable";
        let command = WorkerCommand::boxed().interface();
        let matches = command.get_matches_from(&[
            "worker",
            "-p",
            "mock_pipeline_name",
            "-r",
            "mock_run_id",
            "-e",
            environment_variable_name,
            "-e",
            environment_variable_name,
            "-e",
            environment_variable_name,
        ]);

        assert_eq!(
            matches.get_many::<String>(ENVIRONMENT).map(|v| v.len()),
            Some(3)
        )
    }
}
