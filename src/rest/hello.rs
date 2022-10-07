use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{extract::Query, routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub fn hello_routes() -> Router {
    Router::new().route("/hello", get(hello_handler))
}

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
