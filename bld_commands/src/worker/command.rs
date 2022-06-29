use crate::run::parse_variables;
use crate::BldCommand;
use bld_config::BldConfig;
use bld_core::database::{new_connection_pool, pipeline_runs};
use bld_core::execution::PipelineExecution;
use bld_core::logger::FileLogger;
use bld_core::proxies::ServerPipelineProxy;
use bld_runner::RunnerBuilder;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

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
        let cfg = Arc::new(BldConfig::load()?);
        let pipeline = matches.value_of(PIPELINE).unwrap_or_default();
        let run_id = matches.value_of(RUN_ID).unwrap_or_default();
        let variables = Arc::new(parse_variables(matches, VARIABLES));
        let environment = Arc::new(parse_variables(matches, ENVIRONMENT));
        let pool = Arc::new(new_connection_pool(&cfg.local.db)?);
        let conn = pool.get()?;
        let pipeline_run = pipeline_runs::select_by_id(&conn, run_id)?;
        let start_date_time = pipeline_run.start_date_time;
        let proxy = Arc::new(ServerPipelineProxy::new(cfg.clone(), pool.clone()));
        let logger = Arc::new(Mutex::new(FileLogger::new(cfg.clone(), run_id)?));
        let exec = Arc::new(Mutex::new(PipelineExecution::new(pool, run_id)?));
        let rt = Runtime::new()?;
        rt.block_on(async {
            let runner = RunnerBuilder::default()
                .run_id(run_id)
                .run_start_time(&start_date_time)
                .config(cfg.clone())
                .proxy(proxy.clone())
                .pipeline(pipeline)
                .execution(exec.clone())
                .logger(logger.clone())
                .environment(environment.clone())
                .variables(variables.clone())
                .build()
                .await?;
            runner.run().await.await
        })
    }
}
