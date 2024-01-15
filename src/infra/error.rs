//! Types for reporting errors that happened during a request.
//!
//! If your function interacts with the database or validates user input,
//! you likely want to return a [`ApiResult`].

use super::extract::Json;
use axum::{
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    http::HeaderValue,
    response::IntoResponse,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tower_http::catch_panic::ResponseForPanic;
use utoipa::ToSchema;

/// A standard error response body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    /// A description of the error.
    message: String,
    /// When the error happened.
    timestamp: OffsetDateTime,
}

impl ErrorBody {
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            timestamp: OffsetDateTime::now_utc(),
        }
    }

    /// The error message.
    pub fn message(&self) -> &str {
        self.message.as_ref()
    }

    /// When the error happened.
    pub fn timestamp(&self) -> OffsetDateTime {
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

impl From<bcrypt::BcryptError> for ApiError {
    fn from(e: bcrypt::BcryptError) -> Self {
        ApiError::InternalError(InternalError::BcryptError(e))
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(e: validator::ValidationErrors) -> Self {
        let mut invalid_fields = String::new();
        for (k, v) in e.field_errors() {
            let mut codes = String::new();
            for e in v {
                codes += &format!("{},", e.code);
            }
            let codes = codes.trim_end_matches(',');
            invalid_fields += &format!("{k} ({codes}),");
        }
        let invalid_fields = invalid_fields.trim_end_matches(',');
        ApiError::ClientError(ClientError::UnprocessableEntity(format!(
            "invalid field(s): {invalid_fields}"
        )))
    }
}

/// Errors caused by the client.
/// The client can do something to fix these.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Input validation failed, or some illegal operation was attempted.
    #[error("{0}")]
    BadRequest(String),
    /// Unsupported media type.
    #[error("unsupported media type")]
    UnsupportedMediaType,
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
    /// Validation errors.
    #[error("{0}")]
    UnprocessableEntity(String),
    /// Custom error.
    #[error("{1}")]
    Custom(StatusCode, String),
}

impl Default for ClientError {
    fn default() -> Self {
        Self::BadRequest("Bad Request".to_string())
    }
}

impl From<JsonRejection> for ClientError {
    fn from(value: JsonRejection) -> Self {
        ClientError::Custom(value.status(), value.body_text())
    }
}

impl From<QueryRejection> for ClientError {
    fn from(value: QueryRejection) -> Self {
        ClientError::Custom(value.status(), value.body_text())
    }
}

impl From<PathRejection> for ClientError {
    fn from(value: PathRejection) -> Self {
        ClientError::Custom(value.status(), value.body_text())
    }
}

impl IntoResponse for ClientError {
    fn into_response(self) -> axum::response::Response {
        let msg = self.to_string();
        let status = match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Custom(status, _) => status,
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
    /// An axum extension was not set.
    #[error("missing extension: {0}")]
    MissingExtension(String),
    /// Bcrypt failed to perform some operation.
    #[error("bcrypt error: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),
    /// Reqwest-call failed.
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    /// Integration error.
    #[error("integration error: {0}")]
    IntegrationError(String),
    /// Serde json error.
    #[error("serde json error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    /// Other miscellaneous errors.
    #[error("{0}")]
    Other(String),
}

impl IntoResponse for InternalError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::SqlxError(_) => StatusCode::BAD_GATEWAY,
            Self::IntegrationError(_) => StatusCode::BAD_GATEWAY,
            Self::ReqwestError(e) if e.is_timeout() => StatusCode::GATEWAY_TIMEOUT,
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

/// A handler for converting panics into proper responses for the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanicHandler;

impl ResponseForPanic for PanicHandler {
    type ResponseBody = axum::body::Body;

    fn response_for_panic(
        &mut self,
        _: Box<dyn std::any::Any + Send + 'static>,
    ) -> http::Response<Self::ResponseBody> {
        ApiError::InternalError(InternalError::Other("Panic".to_string())).into_response()
    }
}
