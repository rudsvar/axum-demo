//! Middleware for modifying requests and responses.

use crate::{
    infra::{
        database::DbPool,
        error::{ApiError, ClientError},
    },
    repository::request_repository::NewRequest,
};
use axum::{body::Bytes, middleware::Next, response::IntoResponse};
use http::{Request, Response};
use hyper::Body;
use tower_http::trace::MakeSpan;

static X_REQUEST_ID: &str = "x-request-id";

#[derive(Clone)]
pub(crate) struct MakeRequestIdSpan;

impl<B> MakeSpan<B> for MakeRequestIdSpan {
    fn make_span(&mut self, request: &Request<B>) -> tracing::Span {
        let request_id = request
            .headers()
            .get(X_REQUEST_ID)
            .expect("request id not set")
            .to_str()
            .expect("invalid request id");
        tracing::trace_span!(
            "request",
            request_id = request_id,
            method = %request.method(),
            uri = %request.uri(),
            version = ?request.version(),
        )
    }
}

/// Print and log the request and response.
pub(crate) async fn print_request_response(
    req: hyper::Request<Body>,
    next: Next<Body>,
    db: DbPool,
) -> Result<impl IntoResponse, ApiError> {
    // Print request
    let (parts, body) = req.into_parts();
    let req_bytes = buffer_and_print("Request", body).await?;
    let req = Request::from_parts(parts, Body::from(req_bytes.clone()));
    let host = req
        .headers()
        .get(http::header::HOST)
        .map(|h| h.to_str())
        .transpose()
        .map_err(|e| ClientError::BadRequest(e.to_string()))?
        .map(|str| str.to_string())
        .ok_or_else(|| ClientError::BadRequest("missing host header".to_string()))?;
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    // Perform request
    let res = next.run(req).await;

    // Print response
    let (parts, body) = res.into_parts();
    let res_bytes = buffer_and_print("Response", body).await?;
    let res = Response::from_parts(parts, Body::from(res_bytes.clone()));

    // Log request
    let mut tx = db.begin().await?;
    let new_req = NewRequest {
        host,
        method,
        uri,
        request_body: String::from_utf8(req_bytes.to_vec()).ok(),
        response_body: String::from_utf8(res_bytes.to_vec()).ok(),
        status: res.status().as_u16() as i32,
    };
    let _ = crate::repository::request_repository::create_request(&mut tx, new_req).await?;
    tx.commit().await?;

    Ok(res)
}

/// Read the entire request body stream and store it in memory.
async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, ApiError>
where
    B: axum::body::HttpBody,
    B::Error: std::fmt::Display,
{
    // Try to read stream
    let bytes = hyper::body::to_bytes(body)
        .await
        .map_err(|e| ApiError::ClientError(ClientError::BadRequest(e.to_string())))?;

    // Log if valid text
    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::trace!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}
