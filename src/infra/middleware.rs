//! Middleware for modifying requests and responses.

use crate::{
    api::request::request_repository::{self, NewRequest},
    infra::{
        database::DbPool,
        error::{ApiError, ClientError},
    },
};
use axum::{body::Body, extract::State, middleware::Next, response::IntoResponse};
use bytes::Bytes;
use http::{Request, Response};
use http_body_util::BodyExt;
use hyper::body::Body as _;
use tower_http::trace::MakeSpan;

use super::error::ApiResult;

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
        tracing::info_span!(
            "request",
            request_id = request_id,
            method = %request.method(),
            uri = %request.uri(),
            version = ?request.version(),
        )
    }
}

/// The maximum size of the request body to log.
const MAX_BODY_SIZE: u64 = 8192;

/// Print and log the request and response.
pub(crate) async fn log_request_response(
    State(db): State<DbPool>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, ApiError> {
    // Print request
    let (parts, body) = req.into_parts();
    let req;
    let log_req = match body.size_hint().upper() {
        Some(n) => n <= MAX_BODY_SIZE,
        _ => false,
    };
    let req_string = if log_req {
        let body_bytes = buffer_and_print("Request", body).await?;
        req = Request::from_parts(parts, Body::from(body_bytes.clone()));
        let body_vec = body_bytes.to_vec();
        String::from_utf8(body_vec).ok()
    } else {
        req = Request::from_parts(parts, body);
        None
    };
    let host = req
        .headers()
        .get(http::header::HOST)
        .map(|h| h.to_str())
        .transpose()
        .map_err(|e| ClientError::BadRequest(e.to_string()))?
        .map(|str| str.to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    // Perform request
    let res = next.run(req).await;

    // Print response
    let (parts, body) = res.into_parts();
    let res;
    let log_res = match body.size_hint().upper() {
        Some(n) => n <= MAX_BODY_SIZE,
        _ => false,
    };
    let res_string = if log_res {
        let body_bytes = buffer_and_print("Response", body).await?;
        let body_vec = body_bytes.to_vec();
        res = Response::from_parts(parts, Body::from(body_bytes.clone())).into_response();
        String::from_utf8(body_vec).ok()
    } else {
        res = Response::from_parts(parts, body);
        None
    };
    let status = res.status().as_u16() as i32;

    // Log request asynchronously
    tokio::spawn(async move {
        let new_req = NewRequest {
            host,
            method,
            uri,
            request_body: req_string,
            response_body: res_string,
            status,
        };
        if let Err(e) = store_request(db, &new_req).await {
            tracing::error!("Failed to store request: {}", e);
        }
    });

    Ok(res)
}

/// Store a request in the database.
async fn store_request(db: DbPool, new_req: &NewRequest) -> ApiResult<()> {
    let mut tx = db.begin().await?;
    let _ = request_repository::log_request(&mut tx, new_req).await?;
    tx.commit().await?;
    Ok(())
}

/// Read the entire request body stream and store it in memory.
#[allow(dead_code)]
async fn buffer_and_print(direction: &str, body: Body) -> Result<Bytes, ApiError> {
    // Try to read stream
    let body: Bytes = body.collect().await.unwrap().to_bytes();

    // Log if valid text
    if let Ok(body) = std::str::from_utf8(&body) {
        tracing::trace!("{} body = {:?}", direction, body);
    }

    Ok(body)
}
