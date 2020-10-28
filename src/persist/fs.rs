use crate::os;
use crate::persist::Dumpster;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub struct FileSystemDumpster {
    file_handle: File,
    new_line: String,
}

impl FileSystemDumpster {
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

impl Dumpster for FileSystemDumpster {
    fn dump(&mut self, text: &str) {
        self.write(text);
    }

    fn dumpln(&mut self, text: &str) {
        self.writeln(text);
    }

    fn info(&mut self, text: &str) {
        self.write(text);
    }

    fn error(&mut self, text: &str) {
        self.write(text);
    }
}
