mod auth;
mod config;
mod helpers;
mod hist;
mod high_avail;
mod init;
mod inspect;
mod list;
mod monit;
mod os;
mod persist;
mod push;
mod run;
mod server;
mod stop;
mod types;

use crate::config::definitions::VERSION;
use crate::helpers::term::print_error;
use clap::App;

fn main() {
    let commands = vec![
        auth::AuthCommand::boxed(),
        config::ConfigCommand::boxed(), 
        hist::HistCommand::boxed(),
        init::InitCommand::boxed(),
        inspect::InspectCommand::boxed(),
        list::ListCommand::boxed(),
        monit::MonitCommand::boxed(),
        push::PushCommand::boxed(),
        run::RunCommand::boxed(),
        server::ServerCommand::boxed(),
        stop::StopCommand::boxed(),
    ];

    let cli = App::new("Bld")
        .version(VERSION)
        .about("A simple CI/CD")
        .subcommands(commands
            .iter()
            .map(|c| c.interface())
            .collect::<Vec<App<'static, 'static>>>())
        .get_matches();

    let result = match cli.subcommand() {
        (id, Some(matches)) => commands
            .iter()
            .find(|c| c.id() == id)
            .map(|c| c.exec(matches))
            .unwrap_or_else(|| Ok(())),
        _ => Ok(())
    };

    if let Err(e) = result {
        if let Err(e) = print_error(&e.to_string()) {
            eprintln!("{}", e.to_string());
        }
    }
}
