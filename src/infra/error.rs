//! Types for reporting errors that happened during a request.
//!
//! If your function interacts with the database or validates user input,
//! you likely want to return a [`ApiResult`].

use super::extract::Json;
use axum::{
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use color_eyre::eyre::anyhow;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tonic::{Code, Status};
use tower_http::catch_panic::ResponseForPanic;
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
    InternalError(#[from] color_eyre::eyre::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::ClientError(e) => e.into_response(),
            ApiError::InternalError(e) => {
                tracing::error!("internal error: {}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody::new("internal error".to_string())),
                )
                    .into_response()
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
            e => ApiError::InternalError(e.into()),
        }
    }
}

impl From<bcrypt::BcryptError> for ApiError {
    fn from(e: bcrypt::BcryptError) -> Self {
        ApiError::InternalError(e.into())
    }
}

impl From<lapin::Error> for ApiError {
    fn from(e: lapin::Error) -> Self {
        ApiError::InternalError(e.into())
    }
}

impl From<deadpool_lapin::PoolError> for ApiError {
    fn from(e: deadpool_lapin::PoolError) -> Self {
        ApiError::InternalError(e.into())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::InternalError(e.into())
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(e: validator::ValidationErrors) -> Self {
        let invalid_fields: String = e
            .field_errors()
            .into_iter()
            .map(|(k, v)| {
                let codes: String = v.iter().map(|e| format!("{},", e.code)).collect();
                let codes = codes.trim_end_matches(',');
                format!("{k} ({codes}),")
            })
            .collect();
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

impl From<ApiError> for Status {
    fn from(e: ApiError) -> Self {
        match e {
            ApiError::ClientError(e) => match e {
                ClientError::BadRequest(message) => Status::invalid_argument(message),
                ClientError::UnsupportedMediaType => {
                    Status::invalid_argument("unsupported media type")
                }
                ClientError::Unauthorized => Status::unauthenticated("unauthenticated"),
                ClientError::Forbidden => Status::permission_denied("permission denied"),
                ClientError::NotFound => Status::not_found("resource not found"),
                ClientError::Conflict => Status::already_exists("resource already exists"),
                ClientError::UnprocessableEntity(_) => {
                    Status::invalid_argument("unprocessable entity")
                }
                ClientError::Custom(status, message) => {
                    Status::new(Code::from_i32(status.as_u16() as i32), message)
                }
            },
            ApiError::InternalError(e) => {
                tracing::error!("request failed: {}", e);
                Status::internal("internal error")
            }
        }
    }
}

/// A handler for converting panics into proper responses for the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanicHandler;

impl ResponseForPanic for PanicHandler {
    type ResponseBody = axum::body::BoxBody;

    fn response_for_panic(
        &mut self,
        _: Box<dyn std::any::Any + Send + 'static>,
    ) -> http::Response<Self::ResponseBody> {
        ApiError::InternalError(anyhow!("panic")).into_response()
    }
}
