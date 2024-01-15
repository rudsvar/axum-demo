use axum::Router;
use tower_sessions::{Expiry, PostgresStore, SessionManagerLayer};

use crate::infra::state::AppState;

pub mod index;

/// View routes.
pub fn views(state: AppState, session_store: PostgresStore) -> Router {
    let session_seconds = state.config().server.session_seconds;
    let expiry = Expiry::OnInactivity(time::Duration::seconds(session_seconds as i64));
    let session_layer = SessionManagerLayer::new(session_store).with_expiry(expiry);
    Router::new()
        .nest("/", index::routes())
        .with_state(state)
        .layer(session_layer)
}
