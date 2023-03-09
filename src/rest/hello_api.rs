//! Implementation of the greeting API. An API that returns a greeting based on a query parameter.

use crate::{
    core::greeting::greeting_service,
    infra::{
        extract::{Json, Query},
        state::AppState,
    },
};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

/// The hello API endpoints.
pub fn routes() -> Router<AppState> {
    Router::new().route("/hello", get(hello))
}

/// A name query parameter.
#[derive(Deserialize, IntoParams)]
pub struct GreetingParams {
    name: Option<String>,
}

impl Debug for GreetingParams {
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
    /// Constructs a new greeting.
    pub fn new(greeting: String) -> Self {
        Self { greeting }
    }

    /// Returns the greeting.
    pub fn greeting(&self) -> &str {
        self.greeting.as_ref()
    }
}

/// A handler for requests to the hello endpoint.
#[utoipa::path(
    get,
    path = "/api/hello",
    params(GreetingParams),
    responses(
        (status = 200, description = "Success", body = Greeting),
    )
)]
#[instrument]
pub async fn hello(Query(params): Query<GreetingParams>) -> Json<Greeting> {
    let name = params.name.as_deref().unwrap_or("World");
    Json(Greeting {
        greeting: greeting_service::greet(name),
    })
}

#[cfg(test)]
mod tests {
    use super::Greeting;
    use crate::{
        infra::extract::Query,
        rest::hello_api::{hello, GreetingParams},
    };

    #[sqlx::test]
    async fn hello_without_name_defaults_to_world() {
        let response = hello(Query(GreetingParams { name: None })).await;

        assert_eq!(
            Greeting {
                greeting: "Hello, World!".to_string(),
            },
            response.0
        );
    }

    #[sqlx::test]
    async fn hello_test() {
        let response = hello(Query(GreetingParams {
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
