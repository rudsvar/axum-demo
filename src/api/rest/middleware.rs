//! Middleware for modifying requests and responses.

use axum::{body::Bytes, middleware::Next, response::IntoResponse};
use http::{HeaderValue, Request, Response, StatusCode};
use hyper::Body;
use std::time::Duration;
use tower_http::trace::OnFailure;
use tracing::Instrument;
use uuid::Uuid;

use crate::infra::error::{ApiError, ClientError, InternalError};

static X_REQUEST_ID: &str = "x-request-id";

/// Do not log anything extra on request failure.
#[derive(Clone)]
pub(crate) struct NoopOnFailure;

impl<FailureClass> OnFailure<FailureClass> for NoopOnFailure {
    fn on_failure(&mut self, _: FailureClass, _: Duration, _: &tracing::Span) {
        // Do nothing
    }
}

/// Generates or propagates a request id, and
/// creates a span that includes it and some request information.
pub(crate) async fn request_id_span<B>(
    req: http::Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    // Get from input or generate new id
    let request_id = req
        .headers()
        .get(X_REQUEST_ID)
        .map(|rid| rid.to_str())
        .transpose()
        .map_err(|e| ClientError::BadRequest(e.to_string()))?
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    // Get request fields
    let method = req.method().to_string();
    let target = req.uri().to_string();
    let span = tracing::trace_span!(
        "request",
        request_id = request_id,
        method = method,
        target = target,
    );
    // Instrument further calls
    let mut res = next.run(req).instrument(span).await;
    let request_id =
        HeaderValue::from_str(&request_id).map_err(|e| InternalError::Other(e.to_string()))?;
    res.headers_mut().append(X_REQUEST_ID, request_id);
    Ok(res)
}

pub(crate) async fn print_request_response(
    req: hyper::Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("Request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("Response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody,
    B::Error: std::fmt::Display,
{
    let bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {} body: {}", direction, err),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::trace!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}
