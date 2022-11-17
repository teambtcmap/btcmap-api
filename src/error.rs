use std::{collections::HashMap, fmt::Display};

use actix_web::{http::header::ContentType, HttpResponse, ResponseError};
use reqwest::StatusCode;

#[derive(Debug)]
pub enum Error {
    CLI(String),
    IO(std::io::Error),
    DB(rusqlite::Error),
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Api(ApiError),
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
            Error::Api(err) => err.fmt(f),
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

#[derive(Debug)]
pub struct ApiError {
    pub http_code: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(http_code: u16, message: &str) -> ApiError {
        ApiError {
            http_code: StatusCode::from_u16(http_code).unwrap(),
            message: message.to_string(),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let mut body: HashMap<&str, &str> = HashMap::new();
        body.insert("message", &self.message);

        HttpResponse::build(self.http_code)
            .insert_header(ContentType::json())
            .body(serde_json::to_string_pretty(&body).unwrap() + "\n")
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
