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
    use super::HelloResponse;
    use crate::api::rest::hello::{hello_handler, Name};
    use axum::extract::Query;

    #[sqlx::test]
    async fn hello_test() {
        let response = hello_handler(Query(Name {
            name: "World".to_string(),
        }))
        .await;

        assert_eq!(
            HelloResponse {
                greeting: "Hello, World!".to_string(),
            },
            response.0
        );
    }
}
