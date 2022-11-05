pub trait Scanner {
    fn fetch(&mut self) -> Vec<String>;
}
