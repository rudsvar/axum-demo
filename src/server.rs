//! REST API implementation.
//!
//! # Examples
//!
//! Hello API.
//!
//! ```rust
//! # use axum_demo::feature::hello::hello_api::Greeting;
//! # tokio_test::block_on(async {
//! # let url = axum_demo::server::spawn_app().await;
//! let response = reqwest::get(format!("{}/hello", url)).await.unwrap();
//! assert_eq!(200, response.status());
//! assert_eq!(Greeting::new("Hello, World!".to_string()), response.json::<Greeting>().await.unwrap());
//! # });
//! ```
//!
//! Hello API with name.
//!
//! ```rust
//! # use axum_demo::feature::hello::hello_api::Greeting;
//! # tokio_test::block_on(async {
//! # let url = axum_demo::server::spawn_app().await;
//! let response = reqwest::get(format!("{}/hello?name=Foo", url)).await.unwrap();
//! assert_eq!(200, response.status());
//! assert_eq!(Greeting::new("Hello, Foo!".to_string()), response.json::<Greeting>().await.unwrap());
//! # });
//! ```
//!
//! Create item.
//!
//! ```rust
//! # use axum_demo::feature::hello::hello_api::Greeting;
//! # use axum_demo::feature::item::item_repository::{NewItem};
//! # tokio_test::block_on(async {
//! # let url = axum_demo::server::spawn_app().await;
//! let client = reqwest::ClientBuilder::default().build().unwrap();
//! let new_item = NewItem { name: "Foo".to_string(), description: None };
//! let response = client.post(format!("{}/items", url)).json(new_item).send().await.unwrap();
//! assert_eq!(201, response.status());
//! let item = response.json::<Item>().await.unwrap();
//! let expected = Item { id: 1, name: "Foo".to_string(), description: None };
//! assert_eq!(expected, item);
//! # });
//! ```

use crate::feature::hello::hello_api;
use crate::feature::info::info_api;
use crate::feature::item::item_api;
use crate::feature::url::url_api;
use crate::feature::user::user_api;
use crate::infra::database::DbPool;
use crate::{
    infra::middleware::{log_request_response, MakeRequestIdSpan},
    infra::{
        config::Config,
        error::{InternalError, PanicHandler},
        state::AppState,
    },
};
use axum::{Extension, Json};
use axum::response::Redirect;
use axum::routing::get;
use axum::{error_handling::HandleErrorLayer, response::IntoResponse, Router};
use hyper::header::AUTHORIZATION;
use sqlx::PgPool;
use std::{iter::once, net::TcpListener, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    timeout::TimeoutLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

/// Constructs the full REST API including middleware.
pub fn rest_api(state: AppState) -> Router {
    let db = state.db().clone();

    // Fallible middleware from tower, mapped to infallible response with [`HandleErrorLayer`].
    let tower_middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e| async move {
            InternalError::Other(format!("Tower middleware failed: {e}")).into_response()
        }))
        .concurrency_limit(100);

    // Our API
    Router::new()
        .merge(info_api::routes())
        .merge(hello_api::routes())
        .merge(item_api::routes())
        .merge(user_api::routes())
        .merge(url_api::routes())
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

async fn serve_api(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api)
}

/// Constructs the full axum application.
pub fn app(state: AppState) -> Router {
    // The full application with some top level routes, a GraphQL API, and a REST API.
    let swagger_path = "/swagger-ui";
    Router::new()
        .route("/", get(|| async { Redirect::permanent(swagger_path) }))
        // API specification
        .route("/api.json", get(serve_api));
        // API
        .nest("/api", rest_api(state))
}

/// Starts the axum server.
pub async fn run_app(addr: TcpListener, db: PgPool, config: Config) -> Result<(), hyper::Error> {
    let state = AppState::new(db.clone(), config);
    let app = app(state);

    tracing::info!("Starting axum on {}", addr.local_addr().unwrap());
    axum::Server::from_tcp(addr)?
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown())
        .await
}

/// Completes when when ctrl-c is pressed.
pub(crate) async fn shutdown() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to fetch ctrl_c: {}", e);
    }
    tracing::info!("Shutting down");
}

/// Spawn a server on a random port.
pub async fn spawn_app() -> String {
    let config = crate::infra::config::load_config().unwrap();
    let db = crate::infra::database::init_db(&config.database);
    spawn_app_with_db(db).await
}

/// Spawn a server on a random port with a custom database.
pub async fn spawn_app_with_db(db: DbPool) -> String {
    let address = "127.0.0.1";
    let listener = TcpListener::bind(format!("{address}:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = crate::infra::config::load_config().unwrap();
    tokio::spawn(run_app(listener, db, config));
    format!("http://{address}:{port}/api")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        feature::hello::hello_api::Greeting,
        infra::{database::DbPool, error::ErrorBody, state::AppState},
    };
    use axum::Router;
    use base64::Engine;
    use http::{Request, StatusCode};
    use serde::Deserialize;
    use tower::ServiceExt;

    fn test_app(db: DbPool) -> Router {
        let config = crate::infra::config::load_config().unwrap();
        let state = AppState::new(db, config);
        app(state)
    }

    async fn get<T: for<'a> Deserialize<'a>>(url: &str) -> T {
        let client = reqwest::ClientBuilder::default().build().unwrap();
        client.get(url).send().await.unwrap().json().await.unwrap()
    }

    #[sqlx::test]
    fn hello_gives_correct_response(db: DbPool) {
        let url = spawn_app_with_db(db).await;
        let response: Greeting = get(&format!("{url}/hello?name=World")).await;
        assert_eq!("Hello, World!", response.greeting());
    }

    #[sqlx::test]
    fn non_user_cannot_sign_in(db: DbPool) {
        let url = spawn_app_with_db(db).await;
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
        let url = spawn_app_with_db(db).await;
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
        let url = spawn_app_with_db(db).await;
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
        let url = spawn_app_with_db(db).await;
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
        let url = spawn_app_with_db(db).await;
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
        let url = spawn_app_with_db(db).await;
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
        let url = spawn_app_with_db(db).await;
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
    fn swagger_ui_oneshot(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/swagger-ui/index.html")
            .body(hyper::Body::empty())
            .unwrap();
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

    #[sqlx::test]
    fn shorten_url(db: DbPool) {
        let app = test_app(db);

        // Shorten a new URL
        let auth = base64::engine::general_purpose::STANDARD.encode("user:user");
        let req = Request::post("/api/urls")
            .header("Authorization", format!("Basic {}", &auth))
            .header("Content-Type", "application/json")
            .body(r#"{"name": "example", "target": "https://example.com/"}"#.into())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(StatusCode::CREATED, res.status());

        // Visits the shortened URL
        let req = Request::get("/api/urls/example")
            .body(hyper::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::SEE_OTHER, res.status());
        assert_eq!("https://example.com/", res.headers()["location"]);
    }
}
