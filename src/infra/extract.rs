//! Custom axum extractors.

use super::error::ClientError;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        FromRequest, FromRequestParts,
    },
    response::IntoResponse,
};
use serde::Serialize;

/// A custom JSON extractor since axum's does not let us customize the response.
#[derive(Debug, Clone, Copy, Default, FromRequest)]
#[from_request(via(axum::extract::Json), rejection(ClientError))]
pub struct Json<T>(pub T);

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl From<JsonRejection> for ClientError {
    fn from(value: JsonRejection) -> Self {
        ClientError::Custom(value.status(), value.body_text())
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::extract::Json(self.0).into_response()
    }
}

/// A custom Query extractor since axum's does not let us customize the response.
#[derive(Debug, Clone, Copy, Default, FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ClientError))]
pub struct Query<T>(pub T);

impl<T> AsRef<T> for Query<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl From<QueryRejection> for ClientError {
    fn from(value: QueryRejection) -> Self {
        ClientError::Custom(value.status(), value.body_text())
    }
}
