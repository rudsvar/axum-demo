use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    ClientError(#[from] ClientError),
    #[error("{0}")]
    DatabaseError(#[from] sqlx::Error),
}

impl PartialEq for ApiError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ClientError(l0), Self::ClientError(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::ClientError(e) => e.into_response(),
            e => {
                tracing::error!("internal error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("internal error".to_string())),
                )
                    .into_response()
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ClientError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for ClientError {
    fn into_response(self) -> axum::response::Response {
        let message = self.to_string();
        let status = match self {
            ClientError::Unauthorized => StatusCode::UNAUTHORIZED,
            ClientError::Forbidden => StatusCode::FORBIDDEN,
            ClientError::NotFound => StatusCode::NOT_FOUND,
            ClientError::BadRequest(_) => StatusCode::BAD_REQUEST,
        };
        (status, Json(ErrorResponse::new(message))).into_response()
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    message: String,
}

impl ErrorResponse {
    pub(crate) fn new(message: String) -> Self {
        Self { message }
    }
}
