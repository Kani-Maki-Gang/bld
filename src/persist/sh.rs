use crate::persist::Logger;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub struct ShellLogger;

impl Logger for ShellLogger {
    fn dump(&mut self, text: &str) {
        print!("{}", text);
    }

    fn dumpln(&mut self, text: &str) {
        println!("{}", text);
    }

    fn info(&mut self, text: &str) {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
        let _ = writeln!(&mut stdout, "{}", text);
        let _ = stdout.set_color(ColorSpec::new().set_fg(None));
    }

    fn error(&mut self, text: &str) {
        let mut stderr = StandardStream::stderr(ColorChoice::Always);
        let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
        let _ = writeln!(&mut stderr, "{}", text);
        let _ = stderr.set_color(ColorSpec::new().set_fg(None));
    }
}
