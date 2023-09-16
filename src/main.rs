use std::process::exit;

use anyhow::Result;
use bld_commands::cli::Cli;
use bld_utils::term::print_error;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.invoke().map_err(|e| e.to_string()) {
        Err(e) if !e.is_empty() => {
            print_error(&e)?;
            exit(1)
        }
        _ => Ok(()),
    }
}
