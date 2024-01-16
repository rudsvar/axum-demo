use axum::Router;
use tower_sessions::{Expiry, PostgresStore, SessionManagerLayer};

use crate::infra::state::AppState;

pub mod index;

/// View routes.
pub fn views(state: AppState, session_store: PostgresStore) -> Router {
    let session_duration = state.config().server.session_duration;
    let duration = time::Duration::try_from(session_duration)
        .expect("failed to convert std::time::Duration to time::Duration");
    let expiry = Expiry::OnInactivity(duration);
    tracing::info!("Session expiry: {:?}", expiry);
    let session_layer = SessionManagerLayer::new(session_store).with_expiry(expiry);
    Router::new()
        .nest("/", index::routes())
        .with_state(state)
        .layer(session_layer)
}
