//! APIs for getting information about the application.

use crate::infra::{extract::Json, state::AppState};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The item API endpoints.
pub fn routes() -> Router<AppState> {
    Router::new().route("/info", get(info))
}

/// Application information.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
pub struct AppInfo {
    // The application name.
    name: &'static str,
    // The application version.
    version: &'static str,
}

/// Returns application information.
#[utoipa::path(
    get,
    path = "/api/info",
    responses(
        (status = 200, description = "Success", body = AppInfo),
    )
)]
pub async fn info() -> Json<AppInfo> {
    Json(AppInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}
