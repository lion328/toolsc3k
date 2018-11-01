use std::{error::Error as StdError, fmt, result, io};

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    IXFFile(String),
    RefPackCompression(String),
    Image(String),
    PAKFile(String),
    Other(String),
    OtherError(Box<StdError>),
}

impl fmt::Display for Error {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IO(ref e) => write!(f, "io error: {}", e),
            Error::IXFFile(ref s) => write!(f, "sc3k format error: {}", s),
            Error::RefPackCompression(ref s) => write!(f, "refpack compression error: {}", s),
            Error::Image(ref s) => write!(f, "image format error: {}", s),
            Error::PAKFile(ref s) => write!(f, "pak format error: {}", s),
            Error::Other(ref s) => write!(f, "error: {}", s),
            Error::OtherError(ref e) => write!(f, "error: {}", e),
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
