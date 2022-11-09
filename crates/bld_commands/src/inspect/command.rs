use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use bld_utils::request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tracing::debug;

static INSPECT: &str = "inspect";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";

pub struct InspectCommand;

impl BldCommand for InspectCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        INSPECT
    }

    fn interface(&self) -> Command {
        let pipeline = Arg::new(PIPELINE)
            .long("pipeline")
            .short('p')
            .help("The name of the pipeline to inspect")
            .required(true)
            .action(ArgAction::Set);

        let server = Arg::new(SERVER)
            .long("server")
            .short('s')
            .help("The name of the server from which to inspect the pipeline")
            .action(ArgAction::Set);

        Command::new(INSPECT)
            .about("Inspects the contents of a pipeline on a bld server")
            .version(VERSION)
            .args(&[pipeline, server])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let pip = matches.get_one::<String>(PIPELINE).cloned().unwrap();
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;

        debug!(
            "running {} subcommand with --pipeline: {}, --server: {}",
            INSPECT, pip, server.name
        );

        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/inspect", server.host, server.port);
        let headers = request::headers(&server_auth.name, &server_auth.auth)?;

        debug!("sending http request to {}", url);

        System::new().block_on(async move {
            request::post(url, headers, pip).await.map(|r| {
                println!("{r}");
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_inspect_pipeline_arg_accepts_value() {
        let pipeline_name = "mock_pipeline_name";
        let command = InspectCommand::boxed().interface();
        let matches = command.get_matches_from(&["inspect", "-p", pipeline_name]);

        assert_eq!(
            matches.get_one::<String>(PIPELINE),
            Some(&pipeline_name.to_string())
        );
    }

    #[test]
    fn cli_inspect_server_arg_accepts_value() {
        let server_name = "mock_server_name";
        let command = InspectCommand::boxed().interface();
        let matches =
            command.get_matches_from(&["inspect", "-p", "mockPipeline", "-s", server_name]);

        assert_eq!(
            matches.get_one::<String>(SERVER),
            Some(&server_name.to_string())
        );
    }
}
