//! The item API implementation.

use crate::{
    infra::error::ApiResult,
    repository::item_repository::{Item, NewItem},
    service::item_service,
};
use axum::{
    routing::{get, post},
    Json, Router,
};
use axum_sqlx_tx::Tx;
use sqlx::Postgres;
use tracing::instrument;

/// The item API endpoints.
pub fn item_routes() -> Router {
    Router::new()
        .route("/items", post(create_item))
        .route("/items", get(list_items))
}

/// Creates a new item.
#[utoipa::path(
    post,
    path = "/items",
    request_body = NewItem,
    responses(
        (status = 201, description = "Ok", body = Item),
        (status = 409, description = "Conflict", body = ErrorBody),
        (status = 500, description = "Internal error", body = ErrorBody),
    )
)]
#[instrument(skip(tx))]
async fn create_item(mut tx: Tx<Postgres>, Json(new_item): Json<NewItem>) -> ApiResult<Json<Item>> {
    let item = item_service::create_item(&mut tx, new_item).await?;
    Ok(Json(item))
}

/// Lists all items.
#[utoipa::path(
    get,
    path = "/items",
    responses(
        (status = 200, description = "Success", body = [Item]),
        (status = 500, description = "Internal error", body = ErrorBody),
    )
)]
#[instrument(skip(tx))]
pub async fn list_items(mut tx: Tx<Postgres>) -> ApiResult<Json<Vec<Item>>> {
    let items = item_service::list_items(&mut tx).await?;
    Ok(Json(items))
}

#[cfg(test)]
mod tests {}
