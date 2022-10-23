use crate::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::{definitions::VERSION, BldConfig};
use bld_utils::request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tracing::debug;

const REMOVE: &str = "rm";
const SERVER: &str = "server";
const PIPELINE: &str = "pipeline";

pub struct RemoveCommand;

impl RemoveCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for RemoveCommand {
    fn id(&self) -> &'static str {
        REMOVE
    }

    fn interface(&self) -> Command {
        let server = Arg::new(SERVER)
            .short('s')
            .long(SERVER)
            .help("The name of the bld server")
            .action(ArgAction::Set);

        let pipeline = Arg::new(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("The name of the pipeline")
            .action(ArgAction::Set)
            .required(true);

        Command::new(REMOVE)
            .about("Removes a pipeline from a bld server")
            .version(VERSION)
            .args(&vec![server, pipeline])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        System::new().block_on(async move { do_remove(matches).await })
    }
}

async fn do_remove(matches: &ArgMatches) -> Result<()> {
    let config = BldConfig::load()?;
    let server = config
        .remote
        .server_or_first(matches.get_one::<String>(SERVER))?;
    // using an unwrap here because the pipeline option is requried.
    let pipeline = matches.get_one::<String>(PIPELINE).cloned().unwrap();

    debug!(
        "running {} subcommand with --server: {} and --pipeline: {pipeline}",
        REMOVE, server.name
    );

    let server_auth = config.remote.same_auth_as(server)?;
    let protocol = server.http_protocol();
    let url = format!("{protocol}://{}:{}/remove", server.host, server.port);
    let headers = request::headers(&server_auth.name, &server_auth.auth)?;

    debug!("sending {protocol} request to {url}");
    request::post(url, headers, pipeline).await.map(|r| {
        println!("{r}");
    })
}
