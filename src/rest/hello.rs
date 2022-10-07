use crate::service;
use axum::{extract::Query, routing::get, Json, Router};
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
}

/// A handler for requests to the hello endpoint.
#[instrument]
pub async fn hello_handler(Query(name): Query<Name>) -> Json<HelloResponse> {
    Json(HelloResponse {
        greeting: service::greeter::greet(&name.name),
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
                greeting: "Hello, World!".to_string(),
            },
            response
        );
    }
}
