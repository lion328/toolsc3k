use std::{fmt, result, io};

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    IXFFile(String),
}

impl fmt::Display for Error {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IO(ref e) => write!(f, "io error: {}", e),
            Error::IXFFile(ref s) => write!(f, "sc3k format error: {}", s),
        }
    }
}

impl From<io::Error> for Error {

    fn from(e: io::Error) -> Error {
        Error::IO(e)
    }
}

pub type Result<T> = result::Result<T, Error>;
