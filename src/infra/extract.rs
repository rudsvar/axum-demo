//! Custom axum extractors.

use super::error::ClientError;
use aide::OperationIo;
use axum::{
    extract::{FromRequest, FromRequestParts},
    response::IntoResponse,
};
use serde::Serialize;

/// A custom JSON extractor since axum's does not let us customize the response.
#[derive(Debug, Clone, Copy, Default, FromRequest, OperationIo)]
#[from_request(via(axum::extract::Json), rejection(ClientError))]
#[aide(
    input_with = "axum::Json<T>",
    output_with = "axum::Json<T>",
    json_schema
)]
pub struct Json<T>(pub T);

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::extract::Json(self.0).into_response()
    }
}

/// A custom Query extractor since axum's does not let us customize the response.
#[derive(Debug, Clone, Copy, Default, FromRequestParts, OperationIo)]
#[from_request(via(axum::extract::Query), rejection(ClientError))]
#[aide(input_with = "axum::extract::Query<T>", json_schema)]
pub struct Query<T>(pub T);

impl<T> AsRef<T> for Query<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}
