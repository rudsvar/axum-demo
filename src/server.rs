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

use crate::feature::hello::hello_api;
use crate::feature::home::home_api;
use crate::feature::info::info_api;
use crate::feature::item::item_api;
use crate::feature::url::url_api;
use crate::feature::user::user_api;
use crate::infra::database::DbPool;
use crate::infra::openapi::ApiDoc;
use crate::{
    infra::middleware::{log_request_response, MakeRequestIdSpan},
    infra::{
        config::Config,
        error::{InternalError, PanicHandler},
        state::AppState,
    },
};
use axum::middleware::Next;
use axum::{error_handling::HandleErrorLayer, response::IntoResponse, Router};
use hyper::header::AUTHORIZATION;
use sqlx::PgPool;
use std::{iter::once, time::Duration};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    timeout::TimeoutLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tower_sessions::{Expiry, PostgresStore, SessionManagerLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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
        .layer(axum::middleware::from_fn(move |req, next: Next| {
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
pub fn app(state: AppState, store: PostgresStore) -> Router {
    // The full application with some top level routes, a GraphQL API, and a REST API.
    let swagger_path = "/api";
    let session_seconds = state.config().server.session_seconds;
    let expiry = Expiry::OnInactivity(time::Duration::seconds(session_seconds as i64));
    let session_layer = SessionManagerLayer::new(store).with_expiry(expiry);
    Router::new()
        .merge(home_api::routes())
        .layer(session_layer)
        .with_state(state.clone())
        // Swagger ui
        .merge(SwaggerUi::new(swagger_path).url("/api/openapi.json", ApiDoc::openapi()))
        // API
        .nest("/api", rest_api(state))
}

/// Starts the axum server.
pub async fn run_app(
    addr: TcpListener,
    db: PgPool,
    store: PostgresStore,
    config: Config,
) -> Result<(), hyper::Error> {
    let state = AppState::new(db.clone(), config);
    let app = app(state, store);

    tracing::info!("Starting axum on {}", addr.local_addr().unwrap());
    if let Err(e) = axum::serve(addr, app.into_make_service()).await {
        tracing::error!("Server error: {}", e);
    }
    Ok(())
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
    let listener = TcpListener::bind(format!("{address}:0")).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = crate::infra::config::load_config().unwrap();
    let store = PostgresStore::new(db.clone());
    tokio::spawn(run_app(listener, db, store, config));
    format!("http://{address}:{port}/api")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        feature::hello::hello_api::Greeting,
        infra::{database::DbPool, error::ErrorBody, state::AppState},
    };
    use axum::{body::Body, Router};
    use base64::Engine;
    use futures::StreamExt;
    use http::{Request, StatusCode};
    use serde::Deserialize;
    use tower::ServiceExt;

    fn test_app(db: DbPool) -> Router {
        let config = crate::infra::config::load_config().unwrap();
        let store = PostgresStore::new(db.clone());
        let state = AppState::new(db, config);
        app(state, store)
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
        let req = Request::get("/api/index.html").body(Body::empty()).unwrap();
        let result = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, result.status())
    }

    #[sqlx::test]
    fn hello_oneshot(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/api/hello").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, res.status());
        let body: Vec<u8> = res
            .into_body()
            .into_data_stream()
            .filter_map(|res| std::future::ready(res.ok().map(|b| b.to_vec())))
            .concat()
            .await;
        let greeting: Greeting = serde_json::from_slice(&body).unwrap();
        assert_eq!(Greeting::new("Hello, World!".to_string()), greeting)
    }

    #[sqlx::test]
    fn hello_oneshot2(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/api/hello?name=There")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, res.status());
        let body = res
            .into_body()
            .into_data_stream()
            .filter_map(|res| std::future::ready(res.ok().map(|b| b.to_vec())))
            .concat()
            .await;
        let greeting: Greeting = serde_json::from_slice(&body).unwrap();
        assert_eq!(Greeting::new("Hello, There!".to_string()), greeting)
    }

    #[sqlx::test]
    fn shorten_url(db: DbPool) {
        let app = test_app(db);

        // Shorten a new URL
        let auth = base64::engine::general_purpose::STANDARD.encode("user:user");
        let req: Request<Body> = Request::post("/api/urls")
            .header("Authorization", format!("Basic {}", &auth))
            .header("Content-Type", "application/json")
            .body(r#"{"name": "example", "target": "https://example.com/"}"#.into())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(StatusCode::CREATED, res.status());

        // Visits the shortened URL
        let req = Request::get("/api/urls/example")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::SEE_OTHER, res.status());
        assert_eq!("https://example.com/", res.headers()["location"]);
    }
}
