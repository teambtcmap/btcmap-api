use std::{
    collections::HashMap,
    fmt::Display,
    num::{ParseIntError, TryFromIntError},
};

use actix_web::{http::header::ContentType, HttpResponse, ResponseError};
use deadpool_sqlite::{BuildError, ConfigError, CreatePoolError, InteractError, PoolError};
use reqwest::StatusCode;
use tokio::task::JoinError;

#[derive(Debug)]
pub enum Error {
    CLI(String),
    IO(std::io::Error),
    DB(rusqlite::Error),
    Http(http::Error),
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Api(ApiError),
    DbTableRowNotFound,
    Other(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CLI(err) => write!(f, "{}", err),
            Error::IO(err) => err.fmt(f),
            Error::DB(err) => err.fmt(f),
            Error::Http(err) => err.fmt(f),
            Error::Reqwest(err) => err.fmt(f),
            Error::Serde(err) => err.fmt(f),
            Error::Api(err) => err.fmt(f),
            Error::DbTableRowNotFound => write!(f, "DbTableRowNotFound"),
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
        Error::Serde(error)
    }
}

impl From<Error> for std::io::Error {
    fn from(error: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{error}"))
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Error::Other("Integer casting failed".into())
    }
}

impl From<time::error::Format> for Error {
    fn from(_: time::error::Format) -> Self {
        Error::Other("Time formatting error".into())
    }
}

impl From<JoinError> for Error {
    fn from(_: JoinError) -> Self {
        Error::Other("Join error".into())
    }
}

impl From<InteractError> for Error {
    fn from(_: InteractError) -> Self {
        Error::Other("Interact error".into())
    }
}

impl From<PoolError> for Error {
    fn from(_: PoolError) -> Self {
        Error::Other("Pool error".into())
    }
}

impl From<CreatePoolError> for Error {
    fn from(_: CreatePoolError) -> Self {
        Error::Other("Create pool error".into())
    }
}

impl From<ConfigError> for Error {
    fn from(_: ConfigError) -> Self {
        Error::Other("Pool config error".into())
    }
}

impl From<BuildError> for Error {
    fn from(_: BuildError) -> Self {
        Error::Other("Pool building error".into())
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub http_code: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new<S: AsRef<str>>(http_code: u16, message: S) -> ApiError {
        ApiError {
            http_code: StatusCode::from_u16(http_code).unwrap(),
            message: message.as_ref().to_string(),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let mut body: HashMap<&str, &str> = HashMap::new();
        body.insert("message", &self.message);

        HttpResponse::build(self.http_code)
            .insert_header(ContentType::json())
            .body(serde_json::to_string(&body).unwrap() + "\n")
    }

    fn status_code(&self) -> StatusCode {
        self.http_code
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(error: rusqlite::Error) -> Self {
        ApiError::new(500, &error.to_string())
    }
}

impl From<crate::Error> for ApiError {
    fn from(error: crate::Error) -> Self {
        ApiError::new(500, &error.to_string())
    }
}

impl From<ParseIntError> for ApiError {
    fn from(error: ParseIntError) -> Self {
        ApiError::new(500, &error.to_string())
    }
}
