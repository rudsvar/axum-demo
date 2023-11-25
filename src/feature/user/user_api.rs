//! The user API implementation.

use crate::infra::{
    error::ApiResult,
    extract::Json,
    security::{Admin, Role, User},
    state::AppState,
};
use axum::{routing::get, Router};
use tracing::instrument;

/// The user API endpoints.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/user", get(user))
        .route("/admin", get(admin))
        .route("/custom", get(custom))
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

/// A custom role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomRole;

impl Role for CustomRole {
    fn is_satisfied(role: &[&str]) -> bool {
        role.contains(&"foo") && role.contains(&"bar") || role.contains(&"baz")
    }
}

/// Authenticates user with a custom role.
#[instrument]
pub async fn custom(user: User<CustomRole>) -> ApiResult<Json<i32>> {
    tracing::info!("Custom user logged in");
    Ok(Json(user.id()))
}
