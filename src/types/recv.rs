use anyhow::anyhow;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

pub trait CheckStopSignal {
    fn check_stop_signal(&self) -> anyhow::Result<()>;
}

impl CheckStopSignal for Option<Arc<Mutex<Receiver<bool>>>> {
    fn check_stop_signal(&self) -> anyhow::Result<()> {
        if let Some(comm) = &self {
            let comm = comm.lock().unwrap();
            if let Ok(true) = comm.try_recv() {
                return Err(anyhow!("stop signal sent to thread"));
            }
        }
        Ok(())
    }
}
