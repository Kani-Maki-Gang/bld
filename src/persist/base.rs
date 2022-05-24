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
    fn update_container_id(&mut self, container_id: &str) -> anyhow::Result<()>;
}
