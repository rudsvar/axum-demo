//! REST API implementation.
//!
//! # Examples
//!
//! Hello API.
//!
//! ```rust
//! # use axum_demo::api::hello::hello_api::Greeting;
//! # tokio_test::block_on(async {
//! # let url = axum_demo::app::spawn_app().await;
//! let response = reqwest::get(format!("{}/hello", url)).await.unwrap();
//! assert_eq!(200, response.status());
//! assert_eq!(Greeting::new("Hello, World!".to_string()), response.json::<Greeting>().await.unwrap());
//! # });
//! ```
//!
//! Hello API with name.
//!
//! ```rust
//! # use axum_demo::api::hello::hello_api::Greeting;
//! # tokio_test::block_on(async {
//! # let url = axum_demo::app::spawn_app().await;
//! let response = reqwest::get(format!("{}/hello?name=Foo", url)).await.unwrap();
//! assert_eq!(200, response.status());
//! assert_eq!(Greeting::new("Hello, Foo!".to_string()), response.json::<Greeting>().await.unwrap());
//! # });
//! ```

use std::iter;
use std::time::Duration;

use crate::infra::database::DbPool;
use crate::infra::error::{InternalError, PanicHandler};
use crate::infra::middleware::MakeRequestIdSpan;
use crate::infra::openapi::ApiDoc;
use crate::infra::{config::Config, state::AppState};
use axum::error_handling::HandleErrorLayer;
use axum::response::IntoResponse;
use axum::Router;
use http::header::AUTHORIZATION;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tower_sessions::PostgresStore;
use tracing::Level;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

/// Constructs the full axum application.
pub fn app(state: AppState, session_store: PostgresStore) -> Router {
    // Fallible middleware from tower, mapped to infallible response with [`HandleErrorLayer`].
    let tower_middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e| async move {
            InternalError::Other(format!("Tower middleware failed: {e}")).into_response()
        }))
        .concurrency_limit(500);

    // The full application with views and a REST API.
    Router::new()
        .nest("/", crate::views::views(state.clone(), session_store))
        .merge(SwaggerUi::new("/api/swagger-ui").url("/api/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/api/redoc", ApiDoc::openapi()))
        .merge(RapiDoc::new("/api/openapi.json").path("/api/rapidoc"))
        .nest("/api", crate::api::api(state.clone()))
        // Layers
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::infra::middleware::log_request_response,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(MakeRequestIdSpan)
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(()),
        )
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(SetSensitiveRequestHeadersLayer::new(iter::once(
            AUTHORIZATION,
        )))
        .layer(tower_middleware)
        .layer(CatchPanicLayer::custom(PanicHandler))
}

/// Starts the axum server.
pub async fn run_app(
    addr: TcpListener,
    db: PgPool,
    store: PostgresStore,
    config: Config,
) -> Result<(), hyper::Error> {
    let state = AppState::new(db.clone(), config);
    let app = app(state, store).into_make_service();

    tracing::info!("Starting axum on {}", addr.local_addr().unwrap());
    let exit_result = axum::serve(addr, app)
        // .with_graceful_shutdown(crate::infra::shutdown::shutdown_signal())
        .await;

    match exit_result {
        Ok(_) => tracing::info!("Successfully shut down"),
        Err(e) => tracing::error!("Shutdown failed: {}", e),
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
        api::hello::hello_api::Greeting,
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
        let req = Request::get("/api/swagger-ui/index.html")
            .body(Body::empty())
            .unwrap();
        let result = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, result.status())
    }

    #[sqlx::test]
    fn redoc_oneshot(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/api/redoc").body(Body::empty()).unwrap();
        let result = app.oneshot(req).await.unwrap();
        assert_eq!(StatusCode::OK, result.status())
    }

    #[sqlx::test]
    fn rapidoc_oneshot(db: DbPool) {
        let app = test_app(db);
        let req = Request::get("/api/rapidoc").body(Body::empty()).unwrap();
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
