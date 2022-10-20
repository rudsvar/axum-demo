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
    ValidationError(String),
    #[error("{0}")]
    DbError(#[from] sqlx::Error),
    #[error("unauthorized")]
    Unauthorized,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (message, status) = match self {
            ApiError::ValidationError(e) => (e, StatusCode::BAD_REQUEST),
            ApiError::DbError(e) => match e {
                sqlx::Error::RowNotFound => ("not found".to_string(), StatusCode::NOT_FOUND),
                e => {
                    tracing::error!("database error: {}", e.to_string());
                    (
                        "internal error".to_string(),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            },
            e @ ApiError::Unauthorized => (e.to_string(), StatusCode::UNAUTHORIZED),
        };

        (status, Json(ErrorResponse::new(message))).into_response()
    }
}
