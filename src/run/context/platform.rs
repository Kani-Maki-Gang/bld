use crate::run::{Container, Machine};
use std::fmt::{self, Display, Formatter};

pub enum RunPlatform {
    Local(Machine),
    Docker(Box<Container>),
}

impl Display for RunPlatform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(_) => write!(f, "machine"),
            Self::Docker(container) => write!(f, "docker [ {} ]", container.img),
        }
    }
}
