use crate::BldCommand;
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use bld_sock::clients::MonitClient;
use bld_sock::messages::MonitInfo;
use bld_utils::request::WebSocket;
use clap::{Arg, ArgAction, ArgMatches, Command};
use futures::stream::StreamExt;
use tracing::debug;

static MONIT: &str = "monit";
static PIPELINE_ID: &str = "pipeline-id";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";
static LAST: &str = "last";

struct MonitConnectionInfo {
    server: Option<String>,
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
            .action(ArgAction::SetTrue);

        Command::new(MONIT)
            .about("Connects to a bld server to monitor the execution of a pipeline")
            .version(VERSION)
            .args(&vec![pipeline_id, pipeline, server, last])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let pip_id = matches.get_one::<String>(PIPELINE_ID).cloned();
        let pip_name = matches.get_one::<String>(PIPELINE).cloned();
        let pip_last = matches.get_flag(LAST);
        let server = matches.get_one::<String>(SERVER).cloned();

        debug!(
            "running {} subcommand with --pipeline-id: {:?}, --pipeline: {:?}, --server: {:?}, --last: {}",
            MONIT,
            pip_id,
            pip_name,
            server,
            pip_last
        );

        spawn(MonitConnectionInfo {
            server,
            pip_id,
            pip_name,
            pip_last,
        })
    }
}

async fn request(info: MonitConnectionInfo) -> Result<()> {
    let config = BldConfig::load()?;
    let server = config.remote.server_or_first(info.server.as_ref())?;
    let server_auth = config.remote.same_auth_as(&server)?;
    let url = format!(
        "{}://{}:{}/ws-monit/",
        server.ws_protocol(),
        server.host,
        server.port
    );

    debug!("establishing web socket connection on {}", url);

    let (_, framed) = WebSocket::new(&url)?
        .auth(&server_auth)
        .request()
        .connect()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_monit_id_arg_accepts_value() {
        let pipeline_id = "mock_pipeline_id";
        let command = MonitCommand::boxed().interface();
        let matches = command.get_matches_from(&["monit", "-i", pipeline_id]);

        assert_eq!(
            matches.get_one::<String>(PIPELINE_ID),
            Some(&pipeline_id.to_string())
        )
    }

    #[test]
    fn cli_monit_pipeline_arg_accepts_value() {
        let pipeline_name = "mock_pipeline_name";
        let command = MonitCommand::boxed().interface();
        let matches = command.get_matches_from(&["monit", "-p", pipeline_name]);

        assert_eq!(
            matches.get_one::<String>(PIPELINE),
            Some(&pipeline_name.to_string())
        )
    }

    #[test]
    fn cli_monit_server_arg_accepts_value() {
        let server_name = "mock_server_name";
        let command = MonitCommand::boxed().interface();
        let matches = command.get_matches_from(&["monit", "-s", server_name]);

        assert_eq!(
            matches.get_one::<String>(SERVER),
            Some(&server_name.to_string())
        )
    }

    #[test]
    fn cli_monit_last_arg_is_a_flag() {
        let command = MonitCommand::boxed().interface();
        let matches = command.get_matches_from(&["monit", "--last"]);

        assert!(matches.get_flag(LAST))
    }
}
