//! Custom axum extractors.

use axum::{async_trait, body::HttpBody, extract::FromRequest, response::IntoResponse, BoxError};
use serde::{de::DeserializeOwned, Serialize};

use super::error::{ApiError, ClientError};

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
    type Rejection = ApiError;

    async fn from_request(req: http::Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Json(res): axum::extract::Json<T> =
            axum::extract::Json::from_request(req, state)
                .await
                .map_err(|e| ClientError::BadRequest(e.to_string()))?;
        Ok(Json(res))
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::extract::Json(self.0).into_response()
    }
}
