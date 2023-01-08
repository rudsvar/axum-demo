//! REST API implementations.

use crate::{
    core::item::item_repository,
    infra::state::AppState,
    integration::mq::MqPool,
    rest::middleware::{log_request_response, MakeRequestIdSpan},
    shutdown,
};
use axum::{response::Html, Router};
use hyper::header::AUTHORIZATION;
use sqlx::PgPool;
use std::{iter::once, net::TcpListener, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

pub mod greeting_api;
pub mod integration_api;
pub mod item_api;
pub mod middleware;
pub mod user_api;

// OpenApi configuration.
#[derive(OpenApi)]
#[openapi(
    paths(
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
    axum::response::Html(
        r#"
            <h1>Axum demo</h1>
            <ul>
                <li> <a href="/swagger-ui/">Swagger UI</a> </li>
            </ul>
        "#,
    )
}

/// Starts the axum server.
pub async fn axum_server(addr: TcpListener, db: PgPool, mq: MqPool) -> Result<(), hyper::Error> {
    let state = AppState::new(db.clone(), mq);
    let app = Router::new()
        .route("/", axum::routing::get(index))
        // Swagger ui
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        // API
        .nest(
            "/api",
            Router::<AppState>::new()
                .merge(greeting_api::greeting_routes())
                .merge(item_api::item_routes())
                .merge(user_api::user_routes())
                .merge(integration_api::integration_routes()),
        )
        // Layers
        .layer(axum::middleware::from_fn(move |req, next| {
            log_request_response(req, next, db.clone())
        }))
        .with_state(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(MakeRequestIdSpan)
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(()),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(SetSensitiveRequestHeadersLayer::new(once(AUTHORIZATION)))
        .into_make_service();

    // Create tower service
    let service = ServiceBuilder::new()
        .rate_limit(200, Duration::from_secs(1))
        .concurrency_limit(100)
        .timeout(Duration::from_secs(10))
        .service(app);

    tracing::info!("Starting axum on {:?}", addr.local_addr());

    // Start hyper server
    let axum_server = hyper::Server::from_tcp(addr)?
        .serve(service)
        .with_graceful_shutdown(shutdown("axum"));
    axum_server.await
}

#[cfg(test)]
mod tests {
    use crate::{
        infra::{database::DbPool, error::ErrorBody},
        rest::{axum_server, greeting_api::Greeting},
    };
    use serde::Deserialize;
    use std::net::TcpListener;

    async fn spawn_server(db: DbPool) -> String {
        let address = "127.0.0.1";
        let listener = TcpListener::bind(format!("{}:0", address)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let config = crate::infra::config::load_config().unwrap();
        let conn = crate::integration::mq::init_mq(&config.mq).await.unwrap();
        tokio::spawn(axum_server(listener, db, conn));
        format!("http://{}:{}/api", address, port)
    }

    async fn get<T: for<'a> Deserialize<'a>>(url: &str) -> T {
        let client = reqwest::ClientBuilder::default().build().unwrap();
        client.get(url).send().await.unwrap().json().await.unwrap()
    }

    #[sqlx::test]
    fn hello_gives_correct_response(db: DbPool) {
        let url = spawn_server(db).await;
        let response: Greeting = get(&format!("{}/hello?name=World", url)).await;
        assert_eq!("Hello, World!", response.greeting());
    }

    #[sqlx::test]
    fn non_user_cannot_sign_in(db: DbPool) {
        let url = spawn_server(db).await;
        let client = reqwest::ClientBuilder::default().build().unwrap();
        let response: ErrorBody = client
            .get(&format!("{}/user", url))
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
            .get(&format!("{}/user", url))
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
            .get(&format!("{}/user", url))
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
            .get(&format!("{}/admin", url))
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
            .get(&format!("{}/admin", url))
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
            .get(&format!("{}/user", url))
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
            .get(&format!("{}/admin", url))
            .basic_auth("admin", Some("notadmin"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!("unauthorized", response.message());
    }
}
