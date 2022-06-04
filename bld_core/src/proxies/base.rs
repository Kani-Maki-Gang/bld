use std::path::PathBuf;

pub trait PipelineFileSystemProxy {
    fn path(&self, name: &str) -> anyhow::Result<PathBuf>;
    fn read(&self, name: &str) -> anyhow::Result<String>;
    fn create(&self, name: &str, content: &str) -> anyhow::Result<()>;
    fn remove(&self, name: &str) -> anyhow::Result<()>;
}
