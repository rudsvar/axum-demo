//! Implementation of the integration API. An API that calls another service.

use axum::{routing::get, Extension, Json, Router};
use http::Method;
use sqlx::PgPool;
use tower::Service;
use tracing::instrument;

use crate::{
    infra::error::{ApiError, InternalError},
    integration::client::logging_client,
    repository::item_repository::Item,
};

/// Routes for the integrations API.
pub fn integration_routes() -> Router {
    Router::new().route("/remote-items", get(remote_items))
}

/// A handler for requests to the hello endpoint.
#[utoipa::path(
    get,
    path = "/api/remote-items",
    responses(
        (status = 200, description = "Success", body = [Item]),
    )
)]
#[instrument]
pub async fn remote_items(Extension(db): Extension<PgPool>) -> Result<Json<Vec<Item>>, ApiError> {
    let mut client = logging_client(db);
    let req = reqwest::Request::new(Method::GET, "http://localhost:8080/api/items".parse().unwrap());
    let res = client.call(req).await?;
    let res: Vec<Item> = res.json().await.map_err(InternalError::from)?;
    Ok(Json(res))
}

#[cfg(test)]
mod tests {
    use crate::{api::rest::integration_api::remote_items, infra::database::DbPool};
    use axum::Extension;

    #[sqlx::test]
    async fn it_works(db: DbPool) {
        let response = remote_items(Extension(db)).await;
        assert!(response.is_err())
    }
}
