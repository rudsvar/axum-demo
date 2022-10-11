use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("{0}")]
    ClientError(#[from] ClientError),
    #[error("internal error")]
    InternalError(#[from] InternalError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ClientError {
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum InternalError {
    #[error("database error: {0}")]
    DatabaseError(sqlx::Error),
}

impl IntoResponse for InternalError {
    fn into_response(self) -> axum::response::Response {
        let message = self.to_string();
        let status = match self {
            InternalError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(ErrorResponse::new(message))).into_response()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ErrorResponse {
    message: String,
}

impl ErrorResponse {
    pub(crate) fn new(message: String) -> Self {
        Self { message }
    }
}
