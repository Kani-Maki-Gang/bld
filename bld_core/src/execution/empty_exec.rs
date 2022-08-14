use crate::execution::Execution;
use std::sync::{Arc, Mutex};

pub struct EmptyExec;

impl EmptyExec {
    pub fn atom() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self))
    }
}

impl Execution for EmptyExec {
    fn update_state(&mut self, _state: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn check_stop_signal(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
