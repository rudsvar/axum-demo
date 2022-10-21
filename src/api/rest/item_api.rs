use crate::{
    infra::error::ApiResult,
    repository::item_repository::{Item, NewItem},
    service::item_service,
};
use axum::{Json, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use axum_sqlx_tx::Tx;
use serde::Deserialize;
use sqlx::Postgres;
use tracing::instrument;

pub fn item_routes() -> Router {
    Router::new().typed_post(create_item).typed_get(list_items)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/items")]
pub struct ItemsPath;

/// Creates a new item.
#[instrument(skip(tx))]
async fn create_item(
    _: ItemsPath,
    mut tx: Tx<Postgres>,
    Json(new_item): Json<NewItem>,
) -> ApiResult<Json<Item>> {
    let item = item_service::create_item(&mut tx, new_item).await?;
    Ok(Json(item))
}

/// Lists all items.
#[instrument(skip(tx))]
pub async fn list_items(_: ItemsPath, mut tx: Tx<Postgres>) -> ApiResult<Json<Vec<Item>>> {
    let items = item_service::list_items(&mut tx).await?;
    Ok(Json(items))
}

#[cfg(test)]
mod tests {}
