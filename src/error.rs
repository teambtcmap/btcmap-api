use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    CLI(String),
    IO(std::io::Error),
    DB(rusqlite::Error),
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Other(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CLI(err) => write!(f, "{}", err),
            Error::IO(err) => err.fmt(f),
            Error::DB(err) => err.fmt(f),
            Error::Reqwest(err) => err.fmt(f),
            Error::Serde(err) => err.fmt(f),
            Error::Other(err) => write!(f, "{}", err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IO(error)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::DB(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Reqwest(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Serde(error)
    }
}

impl From<Error> for std::io::Error {
    fn from(error: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{error}"))
    }
}
