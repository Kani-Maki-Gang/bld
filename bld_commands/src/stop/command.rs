use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::BldConfig;
use bld_utils::request;
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
        let headers = request::headers(&server_auth.name, &server_auth.auth)?;

        System::new().block_on(async move {
            request::post(url, headers, id).await.map(|r| {
                println!("{r}");
            })
        })
    }
}
