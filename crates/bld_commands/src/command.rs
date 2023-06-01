use anyhow::Result;
use tracing_subscriber::filter::LevelFilter;

pub trait BldCommand {
    fn exec(self) -> Result<()>;

    fn verbose(&self) -> bool;

    fn tracing_level(&self) -> LevelFilter {
        if self.verbose() {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        }
    }

    fn tracing(&self) {
        tracing_subscriber::fmt()
            .with_max_level(self.tracing_level())
            .init()
    }

    fn invoke(self) -> Result<()>
    where
        Self: Sized,
    {
        self.tracing();
        self.exec()
    }
}
