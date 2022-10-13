use crate::service;
use axum::{extract::Query, Json, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub fn hello_routes() -> Router {
    Router::new().typed_get(hello_handler)
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

#[derive(TypedPath, Deserialize)]
#[typed_path("/hello")]
pub struct HelloPath;

/// A handler for requests to the hello endpoint.
#[instrument]
pub async fn hello_handler(_: HelloPath, Query(name): Query<Name>) -> Json<HelloResponse> {
    Json(HelloResponse {
        greeting: service::greeter::greet(&name.name),
    })
}

#[cfg(test)]
mod tests {
    use super::HelloResponse;
    use crate::api::rest::hello::{hello_handler, HelloPath, Name};
    use axum::extract::Query;

    #[sqlx::test]
    async fn hello_test() {
        let response = hello_handler(
            HelloPath,
            Query(Name {
                name: "World".to_string(),
            }),
        )
        .await;

        assert_eq!(
            HelloResponse {
                greeting: "Hello, World!".to_string(),
            },
            response.0
        );
    }
}
