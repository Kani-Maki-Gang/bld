mod auth;
mod config;
mod helpers;
mod hist;
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
    let matches = App::new("Bld")
        .version(VERSION)
        .about("A simple CI/CD")
        .subcommands(vec![
            auth::command(),
            init::command(),
            inspect::command(),
            hist::command(),
            config::command(),
            run::command(),
            server::command(),
            monit::command(),
            list::command(),
            push::command(),
            stop::command(),
        ])
        .get_matches();

    let result = match matches.subcommand() {
        ("login", Some(matches)) => auth::exec(matches),
        ("init", Some(matches)) => init::exec(matches),
        ("inspect", Some(matches)) => inspect::exec(matches),
        ("hist", Some(matches)) => hist::exec(matches),
        ("config", Some(matches)) => config::exec(matches),
        ("run", Some(matches)) => run::exec(matches),
        ("server", Some(matches)) => server::exec(matches),
        ("monit", Some(matches)) => monit::exec(matches),
        ("ls", Some(matches)) => list::exec(matches),
        ("push", Some(matches)) => push::exec(matches),
        ("stop", Some(matches)) => stop::exec(matches),
        _ => Ok(()),
    };

    if let Err(e) = result {
        if let Err(e) = print_error(&e.to_string()) {
            eprintln!("{}", e.to_string());
        }
    }
}
