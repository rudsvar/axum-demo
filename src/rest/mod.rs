//! REST API implementation.

use crate::graphql::{graphiql, graphql_handler};
use crate::infra::error::ApiError;
use crate::infra::extract::Json;
use crate::{
    graphql::{graphql_item_api::QueryRoot, GraphQlData},
    infra::{
        config::Config,
        error::{InternalError, PanicHandler},
        state::AppState,
    },
    integration::mq::MqPool,
    rest::middleware::{log_request_response, MakeRequestIdSpan},
    shutdown,
};
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::{Info, OpenApi};
use aide::OperationOutput;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use axum::{
    error_handling::HandleErrorLayer, response::IntoResponse, routing::get, Extension, Router,
};
use http::StatusCode;
use hyper::header::AUTHORIZATION;
use sqlx::PgPool;
use std::{iter::once, net::TcpListener, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

pub mod email_api;
pub mod hello_api;
pub mod info_api;
pub mod integration_api;
pub mod item_api;
pub mod middleware;
pub mod user_api;

/// A successful response.
#[derive(Debug)]
pub struct ApiResponse<const STATUS: u16, T>(T);

impl<T> ApiResponse<200, T> {
    /// A 200 Ok response.
    pub fn ok(value: T) -> Self {
        Self(value)
    }
}

impl<T> ApiResponse<201, T> {
    /// A 201 Created response.
    pub fn created(value: T) -> Self {
        ApiResponse(value)
    }
}

impl ApiResponse<204, ()> {
    /// A 204 No Content response.
    pub fn no_content() -> Self {
        ApiResponse(())
    }
}

impl<const STATUS: u16, T: OperationOutput> OperationOutput for ApiResponse<STATUS, T> {
    type Inner = T::Inner;

    fn operation_response(
        ctx: &mut aide::gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Option<aide::openapi::Response> {
        T::operation_response(ctx, operation)
    }

    // fn inferred_responses(
    //     ctx: &mut aide::gen::GenContext,
    //     operation: &mut aide::openapi::Operation,
    // ) -> Vec<(Option<u16>, aide::openapi::Response)> {
    //     T::inferred_responses(ctx, operation)
    // }
}

impl<const STATUS: u16, T: IntoResponse> IntoResponse for ApiResponse<STATUS, T> {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(STATUS).map_err(|e| InternalError::Other(e.to_string()));
        match status {
            Ok(status) => (status, self.0).into_response(),
            Err(e) => e.into_response(),
        }
    }
}

/// Serve the OpenAPI specification.
async fn serve_api(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api)
}

/// Constructs the full REST API including middleware.
pub fn rest_api(state: AppState) -> ApiRouter {
    let db = state.db().clone();

    // Fallible middleware from tower, mapped to infallible response with [`HandleErrorLayer`].
    let tower_middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e| async move {
            InternalError::Other(format!("Tower middleware failed: {e}")).into_response()
        }))
        .concurrency_limit(100);

    // Our API
    ApiRouter::new()
        .merge(info_api::routes())
        .merge(hello_api::routes())
        .merge(item_api::routes())
        .merge(user_api::routes())
        .merge(integration_api::routes())
        .merge(email_api::routes())
        .with_state(state)
        // Layers
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(axum::middleware::from_fn(move |req, next| {
            log_request_response(req, next, db.clone())
        }))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(MakeRequestIdSpan)
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(()),
        )
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(SetSensitiveRequestHeadersLayer::new(once(AUTHORIZATION)))
        .layer(tower_middleware)
        .layer(CatchPanicLayer::custom(PanicHandler))
}

/// Constructs the full axum application.
pub fn app(state: AppState) -> Router {
    // The GraphQL schema
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(GraphQlData::new(state.db().clone()))
        .finish();

    let mut api = OpenApi {
        info: Info {
            title: "Axum demo API".to_string(),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    // The full application with some top level routes, a GraphQL API, and a REST API.
    ApiRouter::new()
        // Index
        .nest_service("/", ServeDir::new("static"))
        // Docs
        .nest_service(
            "/doc",
            ServeDir::new("doc").not_found_service(ServeFile::new("doc/axum_demo/index.html")),
        )
        // GraphQL
        .route("/graphiql", get(graphiql).post(graphql_handler))
        .layer(Extension(schema))
        // Swagger ui
        .route("/api.json", get(serve_api))
        // API
        .nest("/api", rest_api(state))
        .finish_api_with(&mut api, |op| op.default_response::<ApiError>())
        .layer(Extension(api))
}

/// Starts the axum server.
pub async fn axum_server(
    addr: TcpListener,
    db: PgPool,
    mq: MqPool,
    config: Config,
) -> Result<(), hyper::Error> {
    let state = AppState::new(db.clone(), mq, config);
    let app = app(state);

    tracing::info!("Starting axum on {:?}", addr.local_addr());
    axum::Server::from_tcp(addr)?
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown("axum"))
        .await
}

#[cfg(test)]
mod tests {
    use super::{app, axum_server};
    use crate::{
        infra::{database::DbPool, error::ErrorBody, state::AppState},
        rest::hello_api::Greeting,
    };
    use axum::Router;
    use http::{Request, StatusCode};
    use serde::Deserialize;
    use std::net::TcpListener;
    use tower::ServiceExt;

    async fn spawn_server(db: DbPool) -> String {
        let address = "127.0.0.1";
        let listener = TcpListener::bind(format!("{address}:0")).unwrap();
        let port = listener.local_addr().unwrap().port();
        let config = crate::infra::config::load_config().unwrap();
        let mq = crate::integration::mq::init_mq(&config.mq).unwrap();
        tokio::spawn(axum_server(listener, db, mq, config));
        format!("http://{address}:{port}/api")
    }

    fn test_app(db: DbPool) -> Router {
        let config = crate::infra::config::load_config().unwrap();
        let mq = crate::integration::mq::init_mq(&config.mq).unwrap();
        let state = AppState::new(db, mq, config);
        app(state)
    }

    async fn get<T: for<'a> Deserialize<'a>>(url: &str) -> T {
        let client = reqwest::ClientBuilder::default().build().unwrap();
        client.get(url).send().await.unwrap().json().await.unwrap()
    }

    #[sqlx::test]
    fn hello_gives_correct_response(db: DbPool) {
        let url = spawn_server(db).await;
        let response: Greeting = get(&format!("{url}/hello?name=World")).await;
        assert_eq!("Hello, World!", response.greeting());
    }

    #[sqlx::test]
    fn non_user_cannot_sign_in(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: ErrorBody = client
            .get(&format!("{url}/user"))
            .basic_auth("notuser", Some("user"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!("unauthorized", response.message());
    }

    #[sqlx::test]
    fn user_can_access_user_endpoint(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: i32 = client
            .get(&format!("{url}/user"))
            .basic_auth("user", Some("user"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(1, response);
    }

    #[sqlx::test]
    fn user_with_wrong_password_gives_401(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: ErrorBody = client
            .get(&format!("{url}/user"))
            .basic_auth("user", Some("notuser"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!("unauthorized", response.message());
    }

    #[sqlx::test]
    fn user_cannot_access_admin_endpoint(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: ErrorBody = client
            .get(&format!("{url}/admin"))
            .basic_auth("user", Some("user"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!("forbidden", response.message());
    }

    #[sqlx::test]
    fn admin_can_access_admin_endpoint(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: i32 = client
            .get(&format!("{url}/admin"))
            .basic_auth("admin", Some("admin"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(2, response);
    }

    #[sqlx::test]
    fn admin_can_access_user_endpoint(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: i32 = client
            .get(&format!("{url}/user"))
            .basic_auth("admin", Some("admin"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(2, response);
    }

    #[sqlx::test]
    fn admin_with_wrong_password_gives_401(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: ErrorBody = client
            .get(&format!("{url}/admin"))
            .basic_auth("admin", Some("notadmin"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!("unauthorized", response.message());
    }

    #[sqlx::test]
    fn index_oneshot(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/").body(hyper::Body::empty()).unwrap();
        let result = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, result.status())
    }

    #[sqlx::test]
    fn hello_oneshot(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/api/hello")
            .body(hyper::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, res.status());
        let body = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let greeting: Greeting = serde_json::from_slice(&body).unwrap();
        assert_eq!(Greeting::new("Hello, World!".to_string()), greeting)
    }

    #[sqlx::test]
    fn hello_oneshot2(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/api/hello?name=There")
            .body(hyper::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, res.status());
        let body = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let greeting: Greeting = serde_json::from_slice(&body).unwrap();
        assert_eq!(Greeting::new("Hello, There!".to_string()), greeting)
    }
}
