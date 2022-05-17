#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod auth;
mod cli;
mod config;
mod helpers;
mod high_avail;
mod hist;
mod init;
mod inspect;
mod list;
mod monit;
mod os;
mod persist;
mod pull;
mod push;
mod remove;
mod run;
mod server;
mod stop;

use crate::config::definitions::VERSION;
use crate::helpers::term::print_error;
use clap::{App, Arg, ArgMatches};
use tracing_subscriber::filter::LevelFilter;

fn tracing_level(matches: &ArgMatches<'_>) -> LevelFilter {
    match matches.occurrences_of("v") {
        0 => LevelFilter::INFO,
        _ => LevelFilter::DEBUG,
    }
}

fn tracing(matches: &ArgMatches<'_>) {
    tracing_subscriber::fmt()
        .with_max_level(tracing_level(matches))
        .init()
}

fn main() {
    let commands = vec![
        auth::AuthCommand::boxed(),
        config::ConfigCommand::boxed(),
        hist::HistCommand::boxed(),
        init::InitCommand::boxed(),
        inspect::InspectCommand::boxed(),
        list::ListCommand::boxed(),
        remove::RemoveCommand::boxed(),
        monit::MonitCommand::boxed(),
        push::PushCommand::boxed(),
        pull::PullCommand::boxed(),
        run::RunCommand::boxed(),
        server::ServerCommand::boxed(),
        stop::StopCommand::boxed(),
    ];

    let cli = App::new("Bld")
        .version(VERSION)
        .about("A simple CI/CD")
        .subcommands(
            commands
                .iter()
                .map(|c| c.interface())
                .collect::<Vec<App<'static, 'static>>>(),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .required(false)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    tracing(&cli);

    let result = match cli.subcommand() {
        (id, Some(matches)) => commands
            .iter()
            .find(|c| c.id() == id)
            .map(|c| c.exec(matches))
            .unwrap_or_else(|| Ok(())),
        _ => Ok(()),
    };

    if let Err(e) = result {
        if let Err(e) = print_error(&e.to_string()) {
            eprintln!("{e}");
        }
    }
}
