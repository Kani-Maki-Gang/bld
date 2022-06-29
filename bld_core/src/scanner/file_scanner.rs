use crate::scanner::Scanner;
use bld_config::{path, BldConfig};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;

pub struct FileScanner {
    path: PathBuf,
    file_handle: Option<File>,
    _cfg: Arc<BldConfig>
}

impl FileScanner {
    pub fn new(cfg: Arc<BldConfig>, run_id: &str) -> Self {
        Self {
            path: path![&cfg.local.logs, run_id],
            file_handle: None,
            _cfg: cfg
        }
    }

    fn try_open(&mut self) {
        if self.file_handle.is_some() {
            return;
        }
        self.file_handle = match self.path.is_file() {
            true => File::open(&self.path).map(Some).unwrap_or(None),
            false => None,
        };
    }
}

impl Scanner for FileScanner {
    fn fetch(&mut self) -> Vec<String> {
        self.try_open();
        let mut content: Vec<String> = vec![];
        if let Some(file_handle) = &self.file_handle {
            let reader = BufReader::new(file_handle);
            for (_i, line) in reader.lines().enumerate() {
                if let Ok(line) = line {
                    content.push(line);
                }
            }
        }
        content
    }
}
