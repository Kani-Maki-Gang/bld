use crate::logger::Logger;
use std::sync::{Arc, Mutex};

pub struct NullLogger;

impl NullLogger {
    #[allow(dead_code)]
    pub fn atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Logger for NullLogger {
    fn dump(&mut self, _: &str) {}

    fn dumpln(&mut self, _: &str) {}

    fn info(&mut self, _: &str) {}

    fn error(&mut self, _: &str) {}
}
