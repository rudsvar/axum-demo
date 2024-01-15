use std::{iter::once, time::Duration};

use crate::infra::{
    error::{InternalError, PanicHandler},
    middleware::MakeRequestIdSpan,
    state::AppState,
};
use axum::{error_handling::HandleErrorLayer, middleware::Next, response::IntoResponse, Router};
use http::header::AUTHORIZATION;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    timeout::TimeoutLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

pub mod hello;
pub mod info;
pub mod item;
pub mod request;
pub mod url;
pub mod user;

/// Constructs the full REST API including middleware.
pub fn api(state: AppState) -> Router {
    let db = state.db().clone();

    // Fallible middleware from tower, mapped to infallible response with [`HandleErrorLayer`].
    let tower_middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e| async move {
            InternalError::Other(format!("Tower middleware failed: {e}")).into_response()
        }))
        .concurrency_limit(100);

    // Our API
    Router::new()
        // API Routes
        .merge(info::info_api::routes())
        .merge(hello::hello_api::routes())
        .merge(item::item_api::routes())
        .merge(user::user_api::routes())
        .merge(url::url_api::routes())
        .with_state(state)
        // Layers
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(axum::middleware::from_fn(move |req, next: Next| {
            crate::infra::middleware::log_request_response(req, next, db.clone())
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
