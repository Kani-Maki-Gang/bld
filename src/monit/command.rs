use crate::config::{definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::headers;
use crate::helpers::term::print_error;
use crate::monit::MonitClient;
use crate::types::{BldCommand, MonitInfo};
use anyhow::anyhow;
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::stream::StreamExt;
use std::collections::HashMap;

static MONIT: &str = "monit";
static PIPELINE_ID: &str = "pipeline-id";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";
static LAST: &str = "last";

struct MonitConnectionInfo {
    host: String,
    port: i64,
    headers: HashMap<String, String>,
    pip_id: Option<String>,
    pip_name: Option<String>,
    pip_last: bool,
}

pub struct MonitCommand;

impl MonitCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }

    async fn request(info: MonitConnectionInfo) -> anyhow::Result<()> {
        let url = format!("http://{}:{}/ws-monit", info.host, info.port);
        let mut client = Client::new().ws(url);
        for (key, value) in info.headers.iter() {
            client = client.header(&key[..], &value[..]);
        }
        let (_, framed) = client
            .connect()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        let (sink, stream) = framed.split();
        let addr = MonitClient::create(|ctx| {
            MonitClient::add_stream(stream, ctx);
            MonitClient::new(SinkWrite::new(sink, ctx))
        });
        addr.send(MonitInfo::new(info.pip_id, info.pip_name, info.pip_last))
            .await?;
        Ok(())
    }

    fn spawn(info: MonitConnectionInfo) {
        let system = System::new("bld-monit");
        Arbiter::spawn(async move {
            if let Err(e) = Self::request(info).await {
                let _ = print_error(&e.to_string());
                System::current().stop();
            }
        });
        let _ = system.run();
    }
}

impl BldCommand for MonitCommand {
    fn id(&self) -> &'static str {
        MONIT
    }

    fn interface(&self) -> App<'static, 'static> {
        let pipeline_id = Arg::with_name(PIPELINE_ID)
            .short("i")
            .long("pipeline-id")
            .help("The id of the pipeline to monitor. Takes precedence over pipeline")
            .takes_value(true);
        let pipeline = Arg::with_name(PIPELINE)
            .short("p")
            .long("pipeline")
            .help("The name of the pipeline of which to monitor the last run")
            .takes_value(true);
        let server = Arg::with_name(SERVER)
            .short("s")
            .long("server")
            .help("The name of the server to monitor")
            .takes_value(true);
        let last = Arg::with_name(LAST)
            .long("last")
            .help("Monitor the execution of the last invoked pipeline. Takes precedence over pipeline-id and pipeline")
            .takes_value(false);
        SubCommand::with_name(MONIT)
            .about("Connects to a bld server to monitor the execution of a pipeline")
            .version(VERSION)
            .args(&vec![pipeline_id, pipeline, server, last])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let config = BldConfig::load()?;
        let pip_id = matches.value_of(PIPELINE_ID).map(|x| x.to_string());
        let pip_name = matches.value_of(PIPELINE).map(|x| x.to_string());
        let pip_last = matches.is_present(LAST);
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        Self::spawn(MonitConnectionInfo {
            host: srv.host.to_string(),
            port: srv.port,
            headers: headers(name, auth)?,
            pip_id,
            pip_name,
            pip_last,
        });
        Ok(())
    }
}
