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
    ClientError(#[from] ClientError),
    #[error("{0}")]
    InternalError(#[from] InternalError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::ClientError(e) => e.into_response(),
            ApiError::InternalError(e) => {
                tracing::error!("internal error: {}", e);
                e.into_response()
            }
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => ApiError::ClientError(ClientError::NotFound),
            e => ApiError::InternalError(InternalError::SqlxError(e)),
        }
    }
}

impl From<bcrypt::BcryptError> for ApiError {
    fn from(e: bcrypt::BcryptError) -> Self {
        ApiError::InternalError(InternalError::BcryptError(e))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("{0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
}

impl IntoResponse for ClientError {
    fn into_response(self) -> axum::response::Response {
        let msg = self.to_string();
        let status = match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
        };
        (status, Json(ErrorResponse::new(msg))).into_response()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    #[error("{0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("missing extension: {0}")]
    MissingExtension(String),
    #[error("bcrypt error: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),
}

impl IntoResponse for InternalError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("internal error".to_string())),
        )
            .into_response()
    }
}
