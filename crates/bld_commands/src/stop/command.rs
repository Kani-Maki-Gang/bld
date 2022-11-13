use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use bld_utils::request::Request;
use clap::{Arg, ArgAction, ArgMatches, Command};

static STOP: &str = "stop";
static ID: &str = "id";
static SERVER: &str = "server";

pub struct StopCommand;

impl BldCommand for StopCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        STOP
    }

    fn interface(&self) -> Command {
        let id = Arg::new(ID)
            .short('i')
            .long("id")
            .help("The id of a pipeline running on a server")
            .required(true)
            .action(ArgAction::Set);

        let server = Arg::new(SERVER)
            .short('s')
            .long("server")
            .help("The name of the server that the pipeline is running")
            .action(ArgAction::Set);

        Command::new(STOP)
            .about("Stops a running pipeline on a server")
            .version(VERSION)
            .args(&[id, server])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let id = matches.get_one::<String>(ID).cloned().unwrap();

        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;

        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let url = format!("{protocol}://{}:{}/stop", server.host, server.port);
        let request = Request::post(&url).auth(&server_auth);

        System::new().block_on(async move {
            request.send_json(id).await.map(|r: String| {
                println!("{r}");
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_stop_id_arg_accepts_value() {
        let pipeline_id = "mock_pipeline_id";
        let command = StopCommand::boxed().interface();
        let matches = command.get_matches_from(&["stop", "-i", pipeline_id]);

        assert_eq!(
            matches.get_one::<String>(ID),
            Some(&pipeline_id.to_string())
        )
    }

    #[test]
    fn cli_stop_server_arg_accepts_value() {
        let server_name = "mock_server_name";
        let command = StopCommand::boxed().interface();
        let matches =
            command.get_matches_from(&["stop", "-i", "mock_pipeline_id", "-s", server_name]);

        assert_eq!(
            matches.get_one::<String>(SERVER),
            Some(&server_name.to_string())
        )
    }
}
