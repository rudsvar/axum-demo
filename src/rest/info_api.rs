//! APIs for getting information about the application.

use crate::infra::{extract::Json, state::AppState};
use aide::axum::{routing::get, ApiRouter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The item API endpoints.
pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new().api_route("/info", get(info))
}

/// Application information.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct AppInfo {
    // The application name.
    name: &'static str,
    // The application version.
    version: &'static str,
}

/// Returns application information.
pub async fn info() -> Json<AppInfo> {
    Json(AppInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}
