mod config;
mod definitions;
mod init;
mod os;
mod monit;
mod server;
mod term;
mod run;

use crate::definitions::VERSION;
use clap::App;
use term::print_error;

#[actix_web::main]
async fn main() {
    let commands = vec![
        init::command(), 
        run::command(),
        server::command(),
        monit::command()
    ];

    let matches = App::new("Bld")
        .version(VERSION)
        .about("A distributed CI/CD")
        .subcommands(commands)
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
