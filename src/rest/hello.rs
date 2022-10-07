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
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        greeting: format!("Hello {}!", name.name),
        count: prev,
    })
}

#[cfg(test)]
mod tests {
    use crate::rest::axum_server;

    use super::HelloResponse;
    use std::net::TcpListener;

    #[tokio::test]
    async fn hello_test() {
        let addr = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = addr.local_addr().unwrap().port();
        let _ = tokio::spawn(axum_server(addr));
        let response: HelloResponse =
            reqwest::get(format!("http://localhost:{}/hello?name=World", port))
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

        assert_eq!(
            HelloResponse {
                greeting: "Hello World!".to_string(),
                count: 0,
            },
            response
        );
    }
}
