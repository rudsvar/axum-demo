//! Implementation of the integration API. An API that calls another service.

use crate::{
    core::item::item_repository::Item,
    infra::{
        error::{ApiError, InternalError},
        extract::Json,
        state::AppState,
    },
    integration::http::http_client,
};
use axum::{routing::get, Extension, Router};
use http::Method;
use sqlx::PgPool;
use tower::Service;
use tracing::instrument;

/// Routes for the integrations API.
pub fn routes() -> Router<AppState> {
    Router::new().route("/remote-items", get(remote_items))
}

/// A handler for fetching items from a "remote" system.
#[utoipa::path(
    get,
    path = "/api/remote-items",
    responses(
        (status = 200, description = "Success", body = [Item]),
    )
)]
#[instrument]
pub async fn remote_items(Extension(db): Extension<PgPool>) -> Result<Json<Vec<Item>>, ApiError> {
    let mut client = http_client(db);
    let req = reqwest::Request::new(
        Method::GET,
        "http://localhost:8080/api/items".parse().unwrap(),
    );
    let res = client.call(req).await?;
    let res: Vec<Item> = res.json().await.map_err(InternalError::from)?;
    Ok(Json(res))
}

#[cfg(test)]
mod tests {
    use crate::{infra::database::DbPool, rest::integration_api::remote_items};
    use axum::Extension;

    #[sqlx::test]
    async fn it_works(db: DbPool) {
        let response = remote_items(Extension(db)).await;
        assert!(response.is_err())
    }
}
