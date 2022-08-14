pub trait Execution {
    fn update_state(&mut self, state: &str) -> anyhow::Result<()>;
    fn check_stop_signal(&self) -> anyhow::Result<()>;
}
