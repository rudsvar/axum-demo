//! REST API implementation.

use crate::{
    core::item::item_repository,
    graphql::{graphql_item_api::QueryRoot, GraphQlData, GraphQlSchema},
    infra::{
        config::Config,
        error::{ApiError, InternalError},
        extract::Json,
        state::AppState,
    },
    integration::mq::MqPool,
    rest::middleware::{log_request_response, MakeRequestIdSpan},
    shutdown,
};
use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    error_handling::HandleErrorLayer,
    response::{Html, IntoResponse},
    routing::get,
    Extension, Router,
};
use hyper::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{iter::once, net::TcpListener, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::{CatchPanicLayer, ResponseForPanic},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi, ToSchema,
};
use utoipa_swagger_ui::SwaggerUi;

pub mod email_api;
pub mod greeting_api;
pub mod integration_api;
pub mod item_api;
pub mod middleware;
pub mod user_api;

// OpenApi configuration.
#[derive(OpenApi)]
#[openapi(
    paths(
        info,
        greeting_api::greet,
        item_api::create_item,
        item_api::list_items,
        item_api::stream_items,
        user_api::user,
        user_api::admin,
        integration_api::remote_items,
        integration_api::post_to_mq,
        integration_api::read_from_mq,
    ),
    components(
        schemas(
            AppInfo,
            greeting_api::Greeting,
            item_repository::NewItem,
            item_repository::Item,
            integration_api::Message,
            crate::infra::error::ErrorBody
        )
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

/// Security settings
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "basic",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)),
            )
        }
    }
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
            <h1>Axum demo</h1>
            <ul>
                <li> <a href="/doc/axum_demo/index.html">Crate documentation</a> </li>
                <li> <a href="/swagger-ui">Swagger UI</a> </li>
                <li> <a href="/graphiql">GraphiQL IDE</a> </li>
            </ul>
        "#,
    )
}

/// A handler for GraphQL requests.
pub async fn graphql_handler(
    schema: Extension<GraphQlSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// A handler for the GraphQL IDE.
pub async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphiql").finish())
}

/// Application information.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
pub struct AppInfo {
    // The application name.
    name: &'static str,
    // The application version.
    version: &'static str,
}

/// Returns application information.
#[utoipa::path(
    get,
    path = "/api/info",
    responses(
        (status = 200, description = "Success", body = AppInfo),
    )
)]
pub async fn info() -> Json<AppInfo> {
    Json(AppInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// A handler for panics.
#[derive(Clone, Copy)]
struct PanicHandler;

impl ResponseForPanic for PanicHandler {
    type ResponseBody = axum::body::BoxBody;

    fn response_for_panic(
        &mut self,
        _: Box<dyn std::any::Any + Send + 'static>,
    ) -> http::Response<Self::ResponseBody> {
        ApiError::InternalError(InternalError::Other("Panic".to_string())).into_response()
    }
}

/// Constructs the full REST API including middleware.
pub fn rest_api(state: AppState) -> Router {
    let db = state.db().clone();

    // Fallible middleware from tower, mapped to infallible response with [`HandleErrorLayer`].
    let tower_middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e| async move {
            InternalError::Other(format!("Tower middleware failed: {e}")).into_response()
        }))
        .buffer(100)
        .concurrency_limit(100)
        .rate_limit(1, Duration::from_micros(1));

    // Our API
    Router::new()
        .route("/info", get(info))
        .merge(greeting_api::routes())
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

    // The full application with some top level routes, a GraphQL API, and a REST API.
    Router::new()
        .route("/", axum::routing::get(index))
        // Docs
        .nest_service(
            "/doc",
            ServeDir::new("doc").not_found_service(ServeFile::new("doc/axum_demo/index.html")),
        )
        // GraphQL
        .route("/graphiql", get(graphiql).post(graphql_handler))
        .layer(Extension(schema))
        // Swagger ui
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        // API
        .nest("/api", rest_api(state))
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
    use super::app;
    use crate::{
        infra::{database::DbPool, error::ErrorBody, state::AppState},
        rest::{axum_server, greeting_api::Greeting},
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
