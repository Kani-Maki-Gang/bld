use anyhow::Result;
use bld_config::{path, BldConfig};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub enum Logger {
    Empty,
    Shell,
    File { handle: File },
}

impl Logger {
    pub fn empty_atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::Empty))
    }

    pub fn shell_atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::Shell))
    }

    pub fn file_atom(config: Arc<BldConfig>, run_id: &str) -> Result<Arc<Mutex<Self>>> {
        let path = path![&config.local.logs, run_id];
        Ok(Arc::new(Mutex::new(Self::File {
            handle: match path.is_file() {
                true => File::open(&path)?,
                false => File::create(&path)?,
            },
        })))
    }

    pub fn dump(&mut self, text: &str) {
        match self {
            Self::Empty => {}
            Self::Shell => {
                print!("{}", text);
            }
            Self::File { handle } => {
                if let Err(e) = writeln!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }
    }

    pub fn dumpln(&mut self, text: &str) {
        match self {
            Self::Empty => {}
            Self::Shell => {
                println!("{text}");
            }
            Self::File { handle } => {
                if let Err(e) = writeln!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }
    }

    pub fn info(&mut self, text: &str) {
        match self {
            Self::Empty => {}
            Self::Shell => {
                let mut stdout = StandardStream::stdout(ColorChoice::Always);
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                let _ = writeln!(&mut stdout, "{text}");
                let _ = stdout.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                if let Err(e) = writeln!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }
    }

    pub fn error(&mut self, text: &str) {
        match self {
            Self::Empty => {}
            Self::Shell => {
                let mut stderr = StandardStream::stderr(ColorChoice::Always);
                let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                let _ = writeln!(&mut stderr, "{text}");
                let _ = stderr.set_color(ColorSpec::new().set_fg(None));
            }
            Self::File { handle } => {
                if let Err(e) = writeln!(handle, "{text}") {
                    eprintln!("Couldn't write to file: {e}");
                }
            }
        }
    }
}
