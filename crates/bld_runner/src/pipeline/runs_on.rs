use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum RunsOn {
    Machine,
    Docker(String),
}

impl Default for RunsOn {
    fn default() -> Self {
        Self::Machine
    }
}

impl Display for RunsOn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Machine => write!(f, "machine"),
            Self::Docker(image) => write!(f, "docker [ {} ]", image),
        }
    }
}
