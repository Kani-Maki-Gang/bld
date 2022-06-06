use crate::scanner::Scanner;
use anyhow::anyhow;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn file_not_found() -> anyhow::Result<FileScanner> {
    Err(anyhow!("file not found"))
}

pub struct FileScanner {
    file_handle: File,
    _index: usize,
}

impl FileScanner {
    pub fn new(path: &str) -> anyhow::Result<Self> {
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
