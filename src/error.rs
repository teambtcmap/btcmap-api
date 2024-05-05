use actix_web::{
    error::QueryPayloadError, http::StatusCode, HttpRequest, HttpResponse, ResponseError,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Rusqlite(rusqlite::Error),
    Http(http::Error),
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    TimeFormat(time::error::Format),
    OsmApi(String),
    OverpassApi(String),
    DeadpoolPool(deadpool_sqlite::PoolError),
    DeadpoolInteract(deadpool_sqlite::InteractError),
    DeadpoolConfig(deadpool_sqlite::ConfigError),
    DeadpoolBuild(deadpool_sqlite::BuildError),
    HttpBadRequest(String),
    HttpUnauthorized(String),
    HttpNotFound(String),
    HttpConflict(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(err) => err.fmt(f),
            Error::Rusqlite(err) => err.fmt(f),
            Error::Http(err) => err.fmt(f),
            Error::Reqwest(err) => err.fmt(f),
            Error::SerdeJson(err) => err.fmt(f),
            Error::TimeFormat(err) => err.fmt(f),
            Error::OsmApi(err) => err.fmt(f),
            Error::OverpassApi(err) => err.fmt(f),
            Error::DeadpoolPool(err) => err.fmt(f),
            Error::DeadpoolInteract(err) => err.fmt(f),
            Error::DeadpoolConfig(err) => err.fmt(f),
            Error::DeadpoolBuild(err) => err.fmt(f),
            Error::HttpBadRequest(err) => write!(f, "{}", err),
            Error::HttpNotFound(err) => write!(f, "{}", err),
            Error::HttpConflict(err) => write!(f, "{}", err),
            Error::HttpUnauthorized(err) => write!(f, "{}", err),
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
        Error::Rusqlite(error)
    }
}

impl From<http::Error> for Error {
    fn from(error: http::Error) -> Self {
        Error::Http(error)
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

pub fn query_error_handler(err: QueryPayloadError, _req: &HttpRequest) -> actix_web::Error {
    Error::HttpBadRequest(format!("Invalid arguments: {err}")).into()
}

#[derive(Serialize, Deserialize)]
pub struct ApiError {
    pub http_code: u16,
    pub message: String,
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(ApiError {
            http_code: self.status_code().as_u16(),
            message: self.to_string(),
        })
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Error::HttpBadRequest(_) => StatusCode::BAD_REQUEST,
            Error::HttpUnauthorized(_) => StatusCode::UNAUTHORIZED,
            Error::HttpNotFound(_) => StatusCode::NOT_FOUND,
            Error::HttpConflict(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
