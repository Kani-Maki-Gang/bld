use crate::types::Result;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn print_info(text: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    writeln!(&mut stdout, "{}", text)?;

    stdout.set_color(ColorSpec::new().set_fg(None))?;

    Ok(())
}

pub fn print_error(text: &str) -> Result<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Always);

    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    writeln!(&mut stderr, "{}", text)?;

    stderr.set_color(ColorSpec::new().set_fg(None))?;

    Ok(())
}
