use axum::Router;

use crate::infra::state::AppState;

pub mod hello;
pub mod info;
pub mod item;
pub mod request;
pub mod url;
pub mod user;

/// Constructs the full REST API including middleware.
pub fn api(state: AppState) -> Router {
    Router::new()
        .merge(info::info_api::routes())
        .merge(hello::hello_api::routes())
        .merge(item::item_api::routes())
        .merge(user::user_api::routes())
        .merge(url::url_api::routes())
        .with_state(state)
}
