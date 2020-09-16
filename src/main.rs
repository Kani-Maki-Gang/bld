mod definitions;
mod init;
mod options;
mod os;
mod run;

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
        eprintln!("<bld | error> {}", e.to_string());
    }
}
