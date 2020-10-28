pub trait Dumpster {
    fn dump(&mut self, text: &str);
    fn dumpln(&mut self, text: &str);
    fn info(&mut self, text: &str);
    fn error(&mut self, text: &str);
} 