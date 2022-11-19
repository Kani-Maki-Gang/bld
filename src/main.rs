use bld_commands::command::BldCommand;
use bld_commands::cli::Cli;
use bld_utils::term::print_error;
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    match cli.exec().map_err(|e| e.to_string()) {
        Err(e) if !e.is_empty() => {
            if let Err(e) = print_error(&e) {
                eprintln!("{e}");
            }
        }
        _ => {}
    }
}
