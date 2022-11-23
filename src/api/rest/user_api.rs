//! The user API implementation.

use crate::infra::{
    error::ApiResult,
    security::{Admin, User},
};
use axum::{routing::get, Json, Router};
use tracing::instrument;

/// The user API endpoints.
pub fn user_routes() -> Router {
    Router::new()
        .route("/user", get(user))
        .route("/admin", get(admin))
}

/// Authenticates a user.
#[utoipa::path(
    get,
    path = "/api/user",
    responses(
        (status = 200, description = "Ok", body = i32),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 500, description = "Internal error", body = ErrorBody),
    ),
    security(
        ("basic" = [])
    )
)]
#[instrument]
pub async fn user(user: User) -> ApiResult<Json<i32>> {
    tracing::info!("User logged in");
    Ok(Json(user.id()))
}

/// Authenticates an admin user.
#[utoipa::path(
    get,
    path = "/api/admin",
    responses(
        (status = 200, description = "Ok", body = i32),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 500, description = "Internal error", body = ErrorBody),
    ),
    security(
        ("basic" = [])
    )
)]
#[instrument]
pub async fn admin(user: User<Admin>) -> ApiResult<Json<i32>> {
    tracing::info!("Admin logged in");
    Ok(Json(user.id()))
}
