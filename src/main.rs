mod config;
mod definitions;
mod init;
mod options;
mod os;
mod term;
mod run;

use term::print_error;

#[tokio::main]
async fn main() {
    let commands = vec![init::command(), run::command()];

    let matches = options::cli(commands);

    let result = match matches.subcommand() {
        ("init", Some(_)) => init::exec(),
        ("run", Some(matches)) => run::exec(matches).await,
        _ => Ok(()),
    };

    if let Err(e) = result {
        if let Err(e) = print_error(&e.to_string()) {
            eprintln!("{}", e.to_string());
        }
    }
}
