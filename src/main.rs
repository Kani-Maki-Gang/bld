use anyhow::anyhow;
use bld_commands::*;
use bld_config::definitions::VERSION;
use bld_utils::term::print_error;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tracing_subscriber::filter::LevelFilter;

const VERBOSITY: &str = "verbosity";

fn tracing_level(matches: &ArgMatches) -> LevelFilter {
    if matches.get_flag(VERBOSITY) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    }
}

fn tracing(matches: &ArgMatches) {
    tracing_subscriber::fmt()
        .with_max_level(tracing_level(matches))
        .init()
}

fn main() {
    let commands: Vec<Box<dyn BldCommand>> = vec![
        auth::AuthCommand::boxed(),
        config::ConfigCommand::boxed(),
        hist::HistCommand::boxed(),
        init::InitCommand::boxed(),
        inspect::InspectCommand::boxed(),
        list::ListCommand::boxed(),
        remove::RemoveCommand::boxed(),
        monit::MonitCommand::boxed(),
        supervisor::SupervisorCommand::boxed(),
        push::PushCommand::boxed(),
        pull::PullCommand::boxed(),
        run::RunCommand::boxed(),
        server::ServerCommand::boxed(),
        stop::StopCommand::boxed(),
        worker::WorkerCommand::boxed(),
    ];

    let cli = Command::new("Bld")
        .version(VERSION)
        .about("A simple CI/CD")
        .subcommands(
            commands
                .iter()
                .map(|c| c.interface())
                .collect::<Vec<Command>>(),
        )
        .arg(
            Arg::new(VERBOSITY)
                .short('v')
                .help("Sets the level of verbosity")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    tracing(&cli);

    let result = match cli.subcommand() {
        Some((id, matches)) => commands
            .iter()
            .find(|c| c.id() == id)
            .ok_or_else(|| anyhow!("unknown subcommand"))
            .and_then(|c| c.exec(matches)),
        _ => Ok(()),
    };

    match result.map_err(|e| e.to_string()) {
        Err(e) if !e.is_empty() => {
            if let Err(e) = print_error(&e) {
                eprintln!("{e}");
            }
        }
        _ => {}
    }
}
