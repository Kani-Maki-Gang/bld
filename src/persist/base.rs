use std::io;

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
    fn update(&mut self, is_running: bool) -> io::Result<()>;
}
