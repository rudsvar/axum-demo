use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use axum::{
    body::Bytes,
    extract::Query,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use hyper::{Body, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::shutdown;

/// A name query parameter.
#[derive(Debug, Deserialize)]
pub struct Name {
    name: String,
}

/// This is a response to the hello endpoint.
#[derive(Serialize)]
pub struct HelloResponse {
    /// A personal greeting.
    greeting: String,
    /// Request counter.
    count: usize,
}

/// A handler for requests to the hello endpoint.
#[instrument]
pub async fn hello_handler(
    Extension(i): Extension<Arc<AtomicUsize>>,
    Query(name): Query<Name>,
) -> Json<HelloResponse> {
    let prev = i.fetch_add(1, Ordering::SeqCst);
    Json(HelloResponse {
        greeting: name.name,
        count: prev,
    })
}

pub async fn axum_server() -> Result<(), hyper::Error> {
    let app = Router::new()
        .route("/", post(|| async move { "Hello from `POST /`" }))
        .route("/hello", get(hello_handler))
        .layer(middleware::from_fn(print_request_response))
        .layer(Extension(Arc::new(AtomicUsize::new(0))))
        .into_make_service();
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("Starting Axum on {}", addr);
    let axum_server = axum::Server::bind(&addr)
        .serve(app)
        .with_graceful_shutdown(shutdown("axum"));
    axum_server.await
}

async fn print_request_response(
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
