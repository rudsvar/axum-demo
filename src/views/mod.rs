use axum::Router;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

use crate::infra::{config::Config, state::AppState};

pub mod index;
pub mod login;
pub mod logout;

pub(crate) const SESSION_USER_KEY: &str = "user";

/// View routes.
pub fn views(state: AppState, config: Config, store: PostgresStore) -> Router {
    let session_duration = config.server.session_duration;
    let duration = time::Duration::try_from(session_duration)
        .expect("failed to convert std::time::Duration to time::Duration");
    let expiry = Expiry::OnInactivity(duration);
    tracing::info!("Session expiry: {:?}", expiry);
    let session_layer = SessionManagerLayer::new(store).with_expiry(expiry);
    Router::new()
        .merge(index::routes())
        .merge(login::routes())
        .merge(logout::routes())
        .with_state(state)
        .layer(session_layer)
}
