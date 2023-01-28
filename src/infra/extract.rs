//! Custom axum extractors.

use super::error::ErrorBody;
use axum::{
    async_trait,
    body::HttpBody,
    extract::{FromRequest, FromRequestParts},
    response::IntoResponse,
    BoxError,
};
use http::{request::Parts, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

/// A custom JSON extractor since axum's does not let us customize the response.
#[derive(Debug, Clone, Copy, Default)]
pub struct Json<T>(pub T);

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

#[async_trait]
impl<S, B, T> FromRequest<S, B> for Json<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorBody>);

    async fn from_request(req: http::Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let res = axum::extract::Json::from_request(req, state)
            .await
            .map_err(|e| (e.status(), Json(ErrorBody::new(e.body_text()))))?;
        Ok(Json(res.0))
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::extract::Json(self.0).into_response()
    }
}

/// A custom Query extractor since axum's does not let us customize the response.
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl<T> AsRef<T> for Query<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

#[async_trait]
impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorBody>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let res = axum::extract::Query::from_request_parts(parts, state)
            .await
            .map_err(|e| (e.status(), Json(ErrorBody::new(e.body_text()))))?;
        Ok(Query(res.0))
    }
}
