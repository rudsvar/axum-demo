use crate::service;
use axum::{extract::Query, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

pub fn hello_routes() -> Router {
    Router::new().route("/hello", get(hello))
}

/// A name query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct Name {
    name: Option<String>,
}

/// This is a response to the hello endpoint.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Greeting {
    /// A personal greeting.
    greeting: String,
}

impl Greeting {
    pub fn greeting(&self) -> &str {
        self.greeting.as_ref()
    }
}

/// A handler for requests to the hello endpoint.
#[utoipa::path(
    get,
    path = "/hello",
    params(Name),
    responses(
        (status = 200, description = "Success", body = HelloResponse),
    )
)]
#[instrument]
pub async fn hello(Query(name): Query<Name>) -> Json<Greeting> {
    Json(Greeting {
        greeting: service::greet_service::greet(name.name.as_deref().unwrap_or("World")),
    })
}

#[cfg(test)]
mod tests {
    use super::Greeting;
    use crate::api::rest::hello_api::{hello, Name};
    use axum::extract::Query;

    #[sqlx::test]
    async fn hello_test() {
        let response = hello(Query(Name {
            name: Some("World".to_string()),
        }))
        .await;

        assert_eq!(
            Greeting {
                greeting: "Hello, World!".to_string(),
            },
            response.0
        );
    }
}
