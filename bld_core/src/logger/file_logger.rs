use crate::logger::Logger;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct FileLogger {
    file_handle: File,
}

impl FileLogger {
    pub fn new(file_path: &str) -> anyhow::Result<Self> {
        let path = Path::new(file_path);
        let file_handle = match path.is_file() {
            true => File::open(&path)?,
            false => File::create(&path)?,
        };
        Ok(Self { file_handle })
    }

    fn write(&mut self, text: &str) {
        if let Err(e) = writeln!(self.file_handle, "{text}") {
            eprintln!("Couldn't write to file: {e}");
        }
    }

    fn writeln(&mut self, text: &str) {
        if let Err(e) = writeln!(self.file_handle, "{text}") {
            eprintln!("Couldn't write to file: {e}");
        }
    }
}

impl Logger for FileLogger {
    fn dump(&mut self, text: &str) {
        self.write(text);
    }

    fn dumpln(&mut self, text: &str) {
        self.writeln(text);
    }

    fn info(&mut self, text: &str) {
        self.writeln(text);
    }

    fn error(&mut self, text: &str) {
        self.writeln(text);
    }
}
