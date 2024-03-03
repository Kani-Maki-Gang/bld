#[derive(Debug, Clone)]
pub struct RemoteRun {
    pub server: String,
    pub run_id: String,
}

impl RemoteRun {
    pub fn new(server: String, run_id: String) -> Self {
        Self { server, run_id }
    }
}
