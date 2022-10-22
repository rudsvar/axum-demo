use axum::{response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A standard error response body.
#[derive(PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    /// A description of the error.
    message: String,
    /// When the error happened.
    timestamp: DateTime<Utc>,
}

impl ErrorBody {
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            timestamp: Utc::now(),
        }
    }

    pub fn message(&self) -> &str {
        self.message.as_ref()
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
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
            sqlx::Error::Database(e) if e.constraint().is_some() => {
                ApiError::ClientError(ClientError::Conflict)
            }
            e => ApiError::InternalError(InternalError::SqlxError(e)),
        }
    }
}

impl From<axum_sqlx_tx::Error> for ApiError {
    fn from(e: axum_sqlx_tx::Error) -> Self {
        ApiError::InternalError(InternalError::AxumSqlxTxError(e))
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
    #[error("conflict")]
    Conflict,
}

impl IntoResponse for ClientError {
    fn into_response(self) -> axum::response::Response {
        let msg = self.to_string();
        let status = match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
        };
        (status, Json(ErrorBody::new(msg))).into_response()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    #[error("{0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("{0}")]
    AxumSqlxTxError(#[from] axum_sqlx_tx::Error),
    #[error("missing extension: {0}")]
    MissingExtension(String),
    #[error("bcrypt error: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),
}

impl IntoResponse for InternalError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody::new("internal error".to_string())),
        )
            .into_response()
    }
}
