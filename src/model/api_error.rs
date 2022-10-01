use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use rusqlite::Connection;
use std::fmt::Display;
use std::sync::MutexGuard;
use std::sync::PoisonError;

#[derive(serde::Serialize, Debug)]
pub struct ApiError {
    message: String,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::from_u16(500).unwrap())
            .insert_header(ContentType::json())
            .body(serde_json::to_string(self).unwrap())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(500).unwrap()
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(error: rusqlite::Error) -> Self {
        ApiError {
            message: error.to_string(),
        }
    }
}

impl From<PoisonError<MutexGuard<'_, Connection>>> for ApiError {
    fn from(_: PoisonError<MutexGuard<'_, Connection>>) -> Self {
        ApiError {
            message: "Failed to lock database connection".to_string(),
        }
    }
}
