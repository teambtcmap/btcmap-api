use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Other(String),
    IO(std::io::Error),
    Rusqlite(rusqlite::Error),
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    TimeFormat(time::error::Format),
    DeadpoolPool(deadpool_sqlite::PoolError),
    DeadpoolInteract(deadpool_sqlite::InteractError),
    DeadpoolConfig(deadpool_sqlite::ConfigError),
    DeadpoolBuild(deadpool_sqlite::BuildError),
    Parse(time::error::Parse),
    Decode(base64::DecodeError),
    GeoJson(Box<geojson::Error>),
    Staticmap(staticmap::Error),
    Blocking(actix_web::error::BlockingError),
    Matrix(String),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Other(_) => None,
            Error::IO(err) => Some(err),
            Error::Rusqlite(err) => Some(err),
            Error::Reqwest(err) => Some(err),
            Error::SerdeJson(err) => Some(err),
            Error::TimeFormat(err) => Some(err),
            Error::DeadpoolPool(err) => Some(err),
            Error::DeadpoolInteract(err) => Some(err),
            Error::DeadpoolConfig(err) => Some(err),
            Error::DeadpoolBuild(err) => Some(err),
            Error::Parse(err) => Some(err),
            Error::Decode(err) => Some(err),
            Error::GeoJson(err) => Some(err),
            Error::Staticmap(err) => Some(err),
            Error::Blocking(err) => Some(err),
            Error::Matrix(_) => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Other(err) => write!(f, "{}", err),
            Error::Parse(err) => write!(f, "{}", err),
            Error::Decode(err) => write!(f, "{}", err),
            Error::GeoJson(err) => write!(f, "{}", err),
            Error::IO(err) => err.fmt(f),
            Error::Rusqlite(err) => err.fmt(f),
            Error::Reqwest(err) => err.fmt(f),
            Error::SerdeJson(err) => err.fmt(f),
            Error::TimeFormat(err) => err.fmt(f),
            Error::DeadpoolPool(err) => err.fmt(f),
            Error::DeadpoolInteract(err) => err.fmt(f),
            Error::DeadpoolConfig(err) => err.fmt(f),
            Error::DeadpoolBuild(err) => err.fmt(f),
            Error::Staticmap(err) => err.fmt(f),
            Error::Blocking(err) => err.fmt(f),
            Error::Matrix(err) => write!(f, "{}", err),
        }
    }
}

impl From<&str> for Error {
    fn from(str: &str) -> Self {
        Error::Other(str.to_owned())
    }
}

impl From<String> for Error {
    fn from(str: String) -> Self {
        Error::Other(str)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IO(error)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::Rusqlite(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Reqwest(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::SerdeJson(error)
    }
}

impl From<time::error::Format> for Error {
    fn from(error: time::error::Format) -> Self {
        Error::TimeFormat(error)
    }
}

impl From<deadpool_sqlite::PoolError> for Error {
    fn from(error: deadpool_sqlite::PoolError) -> Self {
        Error::DeadpoolPool(error)
    }
}

impl From<deadpool_sqlite::InteractError> for Error {
    fn from(error: deadpool_sqlite::InteractError) -> Self {
        Error::DeadpoolInteract(error)
    }
}

impl From<deadpool_sqlite::ConfigError> for Error {
    fn from(error: deadpool_sqlite::ConfigError) -> Self {
        Error::DeadpoolConfig(error)
    }
}

impl From<deadpool_sqlite::BuildError> for Error {
    fn from(error: deadpool_sqlite::BuildError) -> Self {
        Error::DeadpoolBuild(error)
    }
}

impl From<time::error::Parse> for Error {
    fn from(error: time::error::Parse) -> Self {
        Error::Parse(error)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(error: base64::DecodeError) -> Self {
        Error::Decode(error)
    }
}

impl From<geojson::Error> for Error {
    fn from(error: geojson::Error) -> Self {
        Error::GeoJson(Box::new(error))
    }
}

impl From<staticmap::Error> for Error {
    fn from(error: staticmap::Error) -> Self {
        Error::Staticmap(error)
    }
}

impl From<actix_web::error::BlockingError> for Error {
    fn from(error: actix_web::error::BlockingError) -> Self {
        Error::Blocking(error)
    }
}
