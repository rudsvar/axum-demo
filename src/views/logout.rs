use axum::{response::Redirect, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use tower_sessions::Session;

use crate::infra::{error::ClientError, state::AppState};

use super::index::Index;

pub fn routes() -> Router<AppState> {
    Router::new().typed_get(logout)
}

#[derive(TypedPath)]
#[typed_path("/logout", rejection(ClientError))]
pub struct Logout;

pub async fn logout(_: Logout, session: Session) -> Redirect {
    session.delete().await.unwrap();
    let home = Index.to_string();
    Redirect::to(&home)
}
