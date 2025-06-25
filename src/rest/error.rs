use crate::Error;
use actix_web::{http::StatusCode, web::Json, HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;

pub type RestResult<T, E = RestApiError> = std::result::Result<Json<T>, E>;

#[derive(Debug)]
pub struct RestApiError {
    pub code: RestApiErrorCode,
    pub message: String,
}

impl RestApiError {
    pub fn new(code: RestApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code,
            message: message.into(),
        }
    }

    pub fn not_found() -> Self {
        Self::new(
            RestApiErrorCode::NotFound,
            "Entity with requested ID doesn't exist.",
        )
    }

    pub fn database() -> Self {
        Self::new(
            RestApiErrorCode::Database,
            "Database query failed. Contact BTC Map team to resolve.",
        )
    }
}

#[derive(Debug)]
pub enum RestApiErrorCode {
    NotFound,
    Database,
}

impl fmt::Display for RestApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::fmt::Display for RestApiErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestApiErrorCode::NotFound => write!(f, "not_found"),
            RestApiErrorCode::Database => write!(f, "database"),
        }
    }
}

impl RestApiErrorCode {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for RestApiError {
    fn error_response(&self) -> HttpResponse {
        let body = json!({
            "code": self.code.to_string(),
            "message": self.message,
        });
        HttpResponse::build(self.status_code())
            .content_type("application/json")
            .json(body)
    }

    fn status_code(&self) -> StatusCode {
        self.code.status_code()
    }
}

// TODO remove
impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
