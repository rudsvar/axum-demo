//! Implementation of the greeting API. An API that returns a greeting based on a query parameter.

use crate::core::greeting::greeting_service;

use super::AppState;
use axum::{extract::Query, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

/// The hello API endpoints.
pub fn routes() -> Router<AppState> {
    Router::new().route("/hello", get(greet))
}

/// A name query parameter.
#[derive(Deserialize, IntoParams)]
pub struct Name {
    name: Option<String>,
}

impl Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

/// This is a response to the hello endpoint.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Greeting {
    /// A personal greeting.
    greeting: String,
}

impl Greeting {
    /// Returns the greeting.
    pub fn greeting(&self) -> &str {
        self.greeting.as_ref()
    }
}

/// A handler for requests to the hello endpoint.
#[utoipa::path(
    get,
    path = "/api/greet",
    params(Name),
    responses(
        (status = 200, description = "Success", body = Greeting),
    )
)]
#[instrument]
pub async fn greet(Query(name): Query<Name>) -> Json<Greeting> {
    let name = name.name.as_deref().unwrap_or("World");
    Json(Greeting {
        greeting: greeting_service::greet(name),
    })
}

#[cfg(test)]
mod tests {
    use super::Greeting;
    use crate::rest::greeting_api::{greet, Name};
    use axum::extract::Query;

    #[sqlx::test]
    async fn hello_without_name_defaults_to_world() {
        let response = greet(Query(Name { name: None })).await;

        assert_eq!(
            Greeting {
                greeting: "Hello, World!".to_string(),
            },
            response.0
        );
    }

    #[sqlx::test]
    async fn hello_test() {
        let response = greet(Query(Name {
            name: Some("NotWorld".to_string()),
        }))
        .await;

        assert_eq!(
            Greeting {
                greeting: "Hello, NotWorld!".to_string(),
            },
            response.0
        );
    }
}
