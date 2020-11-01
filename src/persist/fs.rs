use crate::os;
use crate::persist::{Logger, Scanner};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Error, ErrorKind, Write};
use std::path::Path;

pub struct FileLogger {
    file_handle: File,
    new_line: String,
}

impl FileLogger {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let path = Path::new(file_path);

        let file_handle = match path.is_file() {
            true => File::open(&path)?,
            false => File::create(&path)?,
        };
        let new_line = match os::name() {
            os::OSname::Windows => "\r\n",
            _ => "\n",
        }
        .to_string();
        Ok(Self {
            file_handle,
            new_line,
        })
    }

    fn write(&mut self, text: &str) {
        let _ = self.file_handle.write(text.as_bytes());
    }

    fn writeln(&mut self, text: &str) {
        let content = format!("{}{}", text, self.new_line);
        let _ = self.file_handle.write(&content.as_bytes());
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

pub struct FileScanner {
    file_handle: File,
    _index: usize,
}

impl FileScanner {
    pub fn new(path: &str) -> io::Result<Self> {
        let fpath = Path::new(path);
        let file_handle = match fpath.is_file() {
            true => File::open(path)?,
            false => return Err(Error::new(ErrorKind::Other, "could not find file")),
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
