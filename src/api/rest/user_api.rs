use crate::infra::{
    error::ApiResult,
    security::{Admin, User},
};
use axum::{routing::get, Json, Router};
use tracing::instrument;

pub fn user_routes() -> Router {
    Router::new()
        .route("/user", get(user))
        .route("/admin", get(admin))
}

/// Authenticates a user.
#[instrument]
pub async fn user(user: User) -> ApiResult<Json<i32>> {
    tracing::info!("User logged in");
    Ok(Json(user.id()))
}

/// Authenticates an admin user.
#[instrument]
pub async fn admin(user: User<Admin>) -> ApiResult<Json<i32>> {
    tracing::info!("Admin logged in");
    Ok(Json(user.id()))
}
