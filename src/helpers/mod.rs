use std::io::{self, Error, ErrorKind};

pub fn err<T>(text: String) -> io::Result<T> {
    Err(Error::new(ErrorKind::Other, text))
}
