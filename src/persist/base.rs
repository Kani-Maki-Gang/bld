use std::path::PathBuf;

pub trait Logger {
    fn dump(&mut self, text: &str);
    fn dumpln(&mut self, text: &str);
    fn info(&mut self, text: &str);
    fn error(&mut self, text: &str);
}

pub trait Scanner {
    fn fetch(&mut self) -> Vec<String>;
}

pub trait Execution {
    fn update_running(&mut self, is_running: bool) -> anyhow::Result<()>;
}

pub trait PipelineFileSystemProxy {
    fn path(&self, name: &str) -> anyhow::Result<PathBuf>;
    fn read(&self, name: &str) -> anyhow::Result<String>;
    fn create(&self, name: &str, content: &str) -> anyhow::Result<()>;
    fn remove(&self, name: &str) -> anyhow::Result<()>;
}
