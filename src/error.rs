use std::{fmt, result, io};

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    IXFFile(String),
    DBPFCompression(String),
    Other(String),
}

impl fmt::Display for Error {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IO(ref e) => write!(f, "io error: {}", e),
            Error::IXFFile(ref s) => write!(f, "sc3k format error: {}", s),
            Error::DBPFCompression(ref s) => write!(f, "dbpf compression error: {}", s),
            Error::Other(ref s) => write!(f, "error: {}", s),
        }
    }
}

impl From<io::Error> for Error {

    fn from(e: io::Error) -> Error {
        Error::IO(e)
    }
}

impl From<String> for Error {

    fn from(s: String) -> Error {
        Error::Other(s)
    }
}

impl From<&'static str> for Error {

    fn from(s: &'static str) -> Error {
        Error::Other(s.to_string())
    }
}

pub type Result<T> = result::Result<T, Error>;
