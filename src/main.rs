mod definitions;
mod options;
mod os;
mod run;
mod init;

fn main() {
    let commands = vec![
        init::command(),
        run::command(),
    ];

    let matches = options::cli(commands);

    let result = match matches.subcommand() {
        ("init", Some(_)) => init::exec(), 
        ("run", Some(matches)) => run::exec(matches),
        _ => Ok(())
    };

    if let Err(e) = result {
        eprintln!("{}", e.to_string());
    }
}
