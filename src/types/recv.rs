use crate::types::{BldError, Result};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

pub trait CheckStopSignal {
    fn check_stop_signal(&self) -> Result<()>;
}

impl CheckStopSignal for Option<Arc<Mutex<Receiver<bool>>>> {
    fn check_stop_signal(&self) -> Result<()> {
        if let Some(comm) = &self {
            let comm = comm.lock().unwrap();
            if let Ok(true) = comm.try_recv() {
                return Err(BldError::Other("stop signal sent to thread".to_string()));
            }
        }
        Ok(())
    }
}
