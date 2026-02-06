use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("supabase error: {0}")]
    Supabase(String),
    #[error("compute error: {0}")]
    Compute(String),
    #[error("serialization error: {0}")]
    Serde(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("unexpected error: {0}")]
    Unexpected(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Supabase(_) => StatusCode::BAD_GATEWAY,
            AppError::Compute(_) => StatusCode::BAD_GATEWAY,
            AppError::Serde(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = ErrorBody {
            error: self.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Serde(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::Supabase(err.to_string())
    }
}

// Convert AppError to tonic::Status for gRPC responses
impl From<AppError> for tonic::Status {
    fn from(err: AppError) -> Self {
        match err {
            AppError::BadRequest(msg) => tonic::Status::invalid_argument(msg),
            AppError::NotFound(msg) => tonic::Status::not_found(msg),
            AppError::Config(msg) => tonic::Status::internal(msg),
            AppError::Supabase(msg) => tonic::Status::unavailable(msg),
            AppError::Compute(msg) => tonic::Status::internal(msg),
            AppError::Serde(msg) => tonic::Status::internal(msg),
            AppError::Io(msg) => tonic::Status::internal(msg),
            AppError::Unexpected(msg) => tonic::Status::internal(msg),
        }
    }
}
