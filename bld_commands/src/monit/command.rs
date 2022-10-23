use crate::monit::MonitClient;
use crate::BldCommand;
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use awc::http::Version;
use awc::Client;
use bld_config::{definitions::VERSION, BldConfig};
use bld_server::requests::MonitInfo;
use bld_utils::request::headers;
use clap::{Arg, ArgAction, ArgMatches, Command};
use futures::stream::StreamExt;
use std::collections::HashMap;
use tracing::debug;

static MONIT: &str = "monit";
static PIPELINE_ID: &str = "pipeline-id";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";
static LAST: &str = "last";

struct MonitConnectionInfo {
    host: String,
    port: i64,
    protocol: String,
    headers: HashMap<String, String>,
    pip_id: Option<String>,
    pip_name: Option<String>,
    pip_last: bool,
}

pub struct MonitCommand;

impl BldCommand for MonitCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        MONIT
    }

    fn interface(&self) -> Command {
        let pipeline_id = Arg::new(PIPELINE_ID)
            .short('i')
            .long("pipeline-id")
            .help("The id of the pipeline to monitor. Takes precedence over pipeline")
            .action(ArgAction::Set);

        let pipeline = Arg::new(PIPELINE)
            .short('p')
            .long("pipeline")
            .help("The name of the pipeline of which to monitor the last run")
            .action(ArgAction::Set);

        let server = Arg::new(SERVER)
            .short('s')
            .long("server")
            .help("The name of the server to monitor")
            .action(ArgAction::Set);

        let last = Arg::new(LAST)
            .long("last")
            .help("Monitor the execution of the last invoked pipeline. Takes precedence over pipeline-id and pipeline")
            .action(ArgAction::Set);

        Command::new(MONIT)
            .about("Connects to a bld server to monitor the execution of a pipeline")
            .version(VERSION)
            .args(&vec![pipeline_id, pipeline, server, last])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let pip_id = matches.get_one::<String>(PIPELINE_ID).cloned();
        let pip_name = matches.get_one::<String>(PIPELINE).cloned();
        let pip_last = matches.get_flag(LAST);
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;

        debug!(
            "running {} subcommand with --pipeline-id: {:?}, --pipeline: {:?}, --server: {}, --last: {}",
            MONIT,
            pip_id,
            pip_name,
            server.name,
            pip_last
        );

        let server_auth = config.remote.same_auth_as(server)?;

        spawn(MonitConnectionInfo {
            host: server.host.to_string(),
            port: server.port,
            protocol: server.ws_protocol(),
            headers: headers(&server_auth.name, &server_auth.auth)?,
            pip_id,
            pip_name,
            pip_last,
        })
    }
}

async fn request(info: MonitConnectionInfo) -> Result<()> {
    let url = format!("{}://{}:{}/ws-monit/", info.protocol, info.host, info.port);

    debug!("establishing web socket connection on {}", url);

    let client = Client::builder()
        .max_http_version(Version::HTTP_11)
        .finish();
    let mut client = client.ws(url);
    for (key, value) in info.headers.iter() {
        client = client.header(&key[..], &value[..]);
    }
    let (_, framed) = client.connect().await.map_err(|e| anyhow!(e.to_string()))?;

    let (sink, stream) = framed.split();
    let addr = MonitClient::create(|ctx| {
        MonitClient::add_stream(stream, ctx);
        MonitClient::new(SinkWrite::new(sink, ctx))
    });

    debug!(
        "sending data over: {:?} {:?} {}",
        info.pip_id, info.pip_name, info.pip_last
    );

    addr.send(MonitInfo::new(info.pip_id, info.pip_name, info.pip_last))
        .await?;
    Ok(())
}

fn spawn(info: MonitConnectionInfo) -> Result<()> {
    debug!("spawing actix system");
    let sys = System::new();
    let res = sys.block_on(request(info));
    sys.run()?;
    res
}
