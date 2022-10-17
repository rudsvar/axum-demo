use axum::{response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    message: String,
    timestamp: DateTime<Utc>,
}

impl ErrorResponse {
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    ServiceError(#[from] ServiceError),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::ServiceError(e) => e.into_response(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("{0}")]
    ValidationError(String),
    #[error("{0}")]
    DbError(#[from] sqlx::Error),
    #[error("unauthorized")]
    Unauthorized,
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        let message = self.to_string();
        let status = match self {
            ServiceError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ServiceError::DbError(e) => match e {
                sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
                e => {
                    tracing::error!("database error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            },
            ServiceError::Unauthorized => StatusCode::UNAUTHORIZED,
        };

        (status, Json(ErrorResponse::new(message))).into_response()
    }
}
