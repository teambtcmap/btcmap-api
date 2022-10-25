use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::MutexGuard;
use std::sync::PoisonError;

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

impl From<PoisonError<MutexGuard<'_, Connection>>> for ApiError {
    fn from(_: PoisonError<MutexGuard<'_, Connection>>) -> Self {
        ApiError::new(500, "Failed to lock database connection")
    }
}
