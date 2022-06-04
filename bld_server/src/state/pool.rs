use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

pub struct PipelinePool {
    pub senders: Mutex<HashMap<String, Sender<bool>>>,
}

impl PipelinePool {
    pub fn new() -> Self {
        PipelinePool {
            senders: Mutex::new(HashMap::new()),
        }
    }
}
