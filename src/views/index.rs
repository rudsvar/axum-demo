use askama::Template;
use axum::Router;
use axum_extra::routing::{RouterExt, TypedPath};

use crate::infra::{error::ClientError, security::User, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new().typed_get(index)
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    username: String,
}

#[derive(TypedPath)]
#[typed_path("/", rejection(ClientError))]
pub struct Index;

pub async fn index(_: Index, user: User) -> IndexTemplate {
    // Display user information
    IndexTemplate {
        username: user.username().to_string(),
    }
}
