//! The item API implementation.

use super::AppState;
use crate::{
    core::item::{
        item_repository::{Item, NewItem},
        item_service,
    },
    infra::{database::DbPool, error::ApiResult},
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use http::StatusCode;
use tracing::instrument;

/// The item API endpoints.
pub fn item_routes() -> Router<AppState> {
    Router::new()
        .route("/items", post(create_item))
        .route("/items", get(list_items))
}

/// Creates a new item.
#[utoipa::path(
    post,
    path = "/api/items",
    request_body = NewItem,
    responses(
        (status = 201, description = "Ok", body = Item),
        (status = 409, description = "Conflict", body = ErrorBody),
        (status = 500, description = "Internal error", body = ErrorBody),
    )
)]
#[instrument(skip(db))]
async fn create_item(
    db: State<DbPool>,
    Json(new_item): Json<NewItem>,
) -> ApiResult<(StatusCode, Json<Item>)> {
    let mut tx = db.begin().await?;
    let item = item_service::create_item(&mut tx, new_item).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(item)))
}

/// Lists all items.
#[utoipa::path(
    get,
    path = "/api/items",
    responses(
        (status = 200, description = "Success", body = [Item]),
        (status = 500, description = "Internal error", body = ErrorBody),
    )
)]
#[instrument(skip(db))]
pub async fn list_items(db: State<DbPool>) -> ApiResult<Json<Vec<Item>>> {
    let mut tx = db.begin().await?;
    let items = item_service::list_items(&mut tx).await?;
    Ok(Json(items))
}

#[cfg(test)]
mod tests {}
