//! Types for reporting errors that happened during a request.
//!
//! If your function interacts with the database or validates user input,
//! you likely want to return a [`ApiResult`].

use axum::{http::HeaderValue, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A standard error response body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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

    /// The error message.
    pub fn message(&self) -> &str {
        self.message.as_ref()
    }

    /// When the error happened.
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// An error from our API.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// An error caused by the client.
    #[error("{0}")]
    ClientError(#[from] ClientError),
    /// An internal error.
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

/// The result of calling API-related functions.
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

/// Errors caused by the client.
/// The client can do something to fix these.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Input validation failed, or some illegal operation was attempted.
    #[error("{0}")]
    BadRequest(String),
    /// Missing or bad credentials.
    #[error("unauthorized")]
    Unauthorized,
    /// The user is not allowed to access the resource.
    #[error("forbidden")]
    Forbidden,
    /// The resource was not found.
    #[error("not found")]
    NotFound,
    /// The resource already exists.
    #[error("conflict")]
    Conflict,
    /// Integration error.
    #[error("integration with external system failed")]
    IntegrationError,
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
            Self::IntegrationError => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status, Json(ErrorBody::new(msg))).into_response()
    }
}

/// An internal error.
/// The client cannot do anything about this.
#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    /// An [`sqlx`] error.
    #[error("{0}")]
    SqlxError(#[from] sqlx::Error),
    /// An [`axum_sqlx_tx`] error.
    #[error("{0}")]
    AxumSqlxTxError(#[from] axum_sqlx_tx::Error),
    /// An axum extension was not set.
    #[error("missing extension: {0}")]
    MissingExtension(String),
    /// Bcrypt failed to perform some operation.
    #[error("bcrypt error: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),
    /// Reqwest-call failed.
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    /// Other miscellaneous errors.
    #[error("{0}")]
    Other(String),
}

impl IntoResponse for InternalError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::SqlxError(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::AxumSqlxTxError(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut response =
            (status, Json(ErrorBody::new("internal error".to_string()))).into_response();
        response
            .headers_mut()
            .insert("Retry-After", HeaderValue::from_static("5"));
        response
    }
}
