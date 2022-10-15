use crate::BldCommand;
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use awc::http::Version;
use awc::Client;
use bld_config::definitions::{TOOL_DEFAULT_PIPELINE, VERSION};
use bld_config::BldConfig;
use bld_core::logger::Logger;
use bld_runner::{self, RunnerBuilder};
use bld_server::requests::RunInfo;
use bld_server::sockets::ExecClient;
use bld_utils::request::{self, headers};
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::debug;

const RUN: &str = "run";
const PIPELINE: &str = "pipeline";
const SERVER: &str = "server";
const DETACH: &str = "detach";
const VARIABLES: &str = "variables";
const ENVIRONMENT: &str = "environment";

pub struct RunCommand;

impl RunCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for RunCommand {
    fn id(&self) -> &'static str {
        RUN
    }

    fn interface(&self) -> App<'static> {
        let pipeline = Arg::with_name(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("Path to pipeline script")
            .takes_value(true);
        let server = Arg::with_name(SERVER)
            .short('s')
            .long(SERVER)
            .help("The name of the server to run the pipeline")
            .takes_value(true);
        let detach = Arg::with_name(DETACH)
            .short('d')
            .long(DETACH)
            .help("Detaches from the run execution (for server mode runs)");
        let variables = Arg::with_name(VARIABLES)
            .short('v')
            .long(VARIABLES)
            .help("Define values for variables of a pipeline")
            .multiple(true)
            .takes_value(true);
        let environment = Arg::with_name(ENVIRONMENT)
            .short('e')
            .long(ENVIRONMENT)
            .help("Define values for environment variables of a pipeline")
            .multiple(true)
            .takes_value(true);
        SubCommand::with_name(RUN)
            .about("Executes a build pipeline")
            .version(VERSION)
            .args(&[pipeline, server, detach, variables, environment])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let pipeline = matches
            .value_of("pipeline")
            .unwrap_or(TOOL_DEFAULT_PIPELINE)
            .to_string();
        let detach = matches.is_present("detach");
        let env = parse_variables(matches, "environment");
        let vars = parse_variables(matches, "variables");
        match matches.value_of("server") {
            Some(server) => {
                let server = config.remote.server(&server)?;
                let server_auth = config.remote.same_auth_as(server)?;
                debug!(
                    "running {} subcommand with --pipeline: {}, --variables: {:?}, --server: {}",
                    RUN,
                    pipeline,
                    vars,
                    server.name,
                );
                on_server(RunOnServer {
                    host: server.host.clone(),
                    port: server.port,
                    protocol: if detach {
                        server.http_protocol()
                    } else {
                        server.ws_protocol()
                    },
                    headers: headers(&server_auth.name, &server_auth.auth)?,
                    detach,
                    pipeline,
                    environment: env,
                    variables: vars,
                })
            }
            None => {
                debug!(
                    "running {} subcommand with --pipeline: {}, --variables: {:?}",
                    RUN, pipeline, vars
                );
                let rt = Runtime::new()?;
                rt.block_on(async {
                    let runner = RunnerBuilder::default()
                        .config(Arc::new(config))
                        .pipeline(&pipeline)
                        .logger(Logger::shell_atom())
                        .environment(Arc::new(env))
                        .variables(Arc::new(vars))
                        .build()
                        .await?;
                    runner.run().await.await
                })
            }
        }
    }
}

pub fn parse_variables(matches: &ArgMatches, arg: &str) -> HashMap<String, String> {
    matches
        .values_of(arg)
        .map(|variable| {
            variable
                .map(|v| {
                    let mut split = v.split('=');
                    let name = split.next().unwrap_or("").to_string();
                    let value = split.next().unwrap_or("").to_string();
                    (name, value)
                })
                .collect::<HashMap<String, String>>()
        })
        .or_else(|| Some(HashMap::new()))
        .unwrap()
}

pub struct RunOnServer {
    pub host: String,
    pub port: i64,
    pub protocol: String,
    pub headers: HashMap<String, String>,
    pub detach: bool,
    pub pipeline: String,
    pub environment: HashMap<String, String>,
    pub variables: HashMap<String, String>,
}

async fn send_run_request(data: RunOnServer) -> Result<()> {
    let url = format!("{}://{}:{}/run", data.protocol, data.host, data.port);

    debug!("sending request to {url}");

    let request_data = RunInfo::new(
        &data.pipeline,
        Some(data.environment.clone()),
        Some(data.variables.clone()),
    );
    request::post(url, data.headers.clone(), request_data)
        .await
        .map(|_| {
            println!("pipeline has been scheduled to run");
        })
}

async fn connect_to_exec_socket(data: RunOnServer) -> Result<()> {
    let url = format!("{}://{}:{}/ws-exec/", data.protocol, data.host, data.port);

    debug!("establishing web socker connection on {}", url);

    let client = Client::builder()
        .max_http_version(Version::HTTP_11)
        .finish();
    let mut client = client.ws(url);
    for (key, value) in data.headers.iter() {
        client = client.header(&key[..], &value[..]);
    }

    let (_, framed) = client.connect().await.map_err(|e| anyhow!(e.to_string()))?;
    let (sink, stream) = framed.split();
    let addr = ExecClient::create(|ctx| {
        ExecClient::add_stream(stream, ctx);
        ExecClient::new(SinkWrite::new(sink, ctx))
    });

    debug!(
        "sending data over: {:?} {:?}",
        data.pipeline, data.variables
    );

    addr.send(RunInfo::new(
        &data.pipeline,
        Some(data.environment.clone()),
        Some(data.variables.clone()),
    ))
    .await
    .map_err(|e| anyhow!(e))
}

pub fn on_server(data: RunOnServer) -> Result<()> {
    debug!("spawing actix system");
    if data.detach {
        System::new().block_on(async move { send_run_request(data).await })
    } else {
        let sys = System::new();
        let res = sys.block_on(async move { connect_to_exec_socket(data).await });
        sys.run()?;
        res
    }
}
