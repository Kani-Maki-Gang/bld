use crate::persist::{Logger, Scanner};
use crate::types::{BldError, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

fn file_not_found() -> Result<FileScanner> {
    let message = String::from("file not found");
    Err(BldError::Other(message))
}

pub struct FileLogger {
    file_handle: File,
}

impl FileLogger {
    pub fn new(file_path: &str) -> Result<Self> {
        let path = Path::new(file_path);
        let file_handle = match path.is_file() {
            true => File::open(&path)?,
            false => File::create(&path)?,
        };
        Ok(Self { file_handle })
    }

    fn write(&mut self, text: &str) {
        if let Err(e) = write!(self.file_handle, "{}", text) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }

    fn writeln(&mut self, text: &str) {
        if let Err(e) = writeln!(self.file_handle, "{}", text) {
            eprintln!("Couldn't write to file: {}", e);
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

pub struct NullLogger;

impl NullLogger {
    pub fn atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Logger for NullLogger {
    fn dump(&mut self, _: &str) {}

    fn dumpln(&mut self, _: &str) {}

    fn info(&mut self, _: &str) {}

    fn error(&mut self, _: &str) {}
}

pub struct FileScanner {
    file_handle: File,
    _index: usize,
}

impl FileScanner {
    pub fn new(path: &str) -> Result<Self> {
        let fpath = Path::new(path);
        let file_handle = match fpath.is_file() {
            true => File::open(path)?,
            false => return file_not_found(),
        };
        Ok(Self {
            file_handle,
            _index: 0,
        })
    }
}

impl Scanner for FileScanner {
    fn fetch(&mut self) -> Vec<String> {
        let mut content = Vec::<String>::new();
        let reader = BufReader::new(&self.file_handle);
        for (_i, line) in reader.lines().enumerate() {
            if let Ok(line) = line {
                content.push(line);
            }
        }
        content
    }
}
