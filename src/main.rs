mod config;
mod definitions;
mod init;
mod monit;
mod os;
mod run;
mod server;
mod term;

use crate::definitions::VERSION;
use clap::App;
use term::print_error;

#[actix_web::main]
async fn main() {
    let matches = App::new("Bld")
        .version(VERSION)
        .about("A distributed CI/CD")
        .subcommands(vec![
            init::command(),
            run::command(),
            server::command(),
            monit::command(),
        ])
        .get_matches();

    let result = match matches.subcommand() {
        ("init", Some(_)) => init::exec(),
        ("run", Some(matches)) => run::exec(matches).await,
        ("server", Some(matches)) => server::exec(matches).await,
        ("monit", Some(matches)) => monit::exec(matches),
        _ => Ok(()),
    };

    if let Err(e) = result {
        if let Err(e) = print_error(&e.to_string()) {
            eprintln!("{}", e.to_string());
        }
    }
}
