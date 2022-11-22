pub trait BldCommand {
    fn exec(self) -> anyhow::Result<()>;
}
