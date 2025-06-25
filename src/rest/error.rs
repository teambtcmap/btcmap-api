use crate::Error;
use actix_web::{
    error::QueryPayloadError, http::StatusCode, web::Json, HttpResponse, ResponseError,
};
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
    InvalidInput,
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
            RestApiErrorCode::InvalidInput => write!(f, "invalid_input"),
            RestApiErrorCode::NotFound => write!(f, "not_found"),
            RestApiErrorCode::Database => write!(f, "database"),
        }
    }
}

impl RestApiErrorCode {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidInput => StatusCode::BAD_REQUEST,
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

impl From<QueryPayloadError> for RestApiError {
    fn from(err: QueryPayloadError) -> Self {
        match err {
            QueryPayloadError::Deserialize(e) => RestApiError {
                code: RestApiErrorCode::InvalidInput,
                message: format!("Invalid query parameters: {}", e),
            },
            _ => RestApiError {
                code: RestApiErrorCode::InvalidInput,
                message: "Invalid query parameters".to_string(),
            },
        }
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
