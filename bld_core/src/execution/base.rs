pub trait Execution {
    fn update_running(&mut self, is_running: bool) -> anyhow::Result<()>;
}
