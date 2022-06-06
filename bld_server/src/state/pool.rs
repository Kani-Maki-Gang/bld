use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

#[derive(Default)]
pub struct PipelinePool {
    pub senders: Mutex<HashMap<String, Sender<bool>>>,
}
