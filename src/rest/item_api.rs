//! The item API implementation.

use crate::{
    core::item::{
        item_repository::{Item, NewItem},
        item_service,
    },
    infra::{
        database::DbPool,
        error::{ApiError, ApiResult, ClientError},
        extract::{Json, Query},
        state::AppState,
        validation::Valid,
    },
};
use aide::axum::{
    routing::{delete, get, post, put},
    ApiRouter,
};
use axum::extract::{Path, State};
use axum_extra::{json_lines::AsResponse, response::JsonLines};
use futures::Stream;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;

use super::ApiResponse;

/// The item API endpoints.
pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route("/items", post(create_item))
        .api_route("/items/:id", get(get_item))
        .api_route("/items/:id", put(update_item))
        .api_route("/items/:id", delete(delete_item))
        .api_route("/items", get(list_items))
        .route("/items2", axum::routing::get(stream_items))
}

/// The id of an item.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
pub struct ItemId {
    id: i32,
}

/// Creates a new item.
#[instrument(skip_all, fields(new_item))]
async fn create_item(
    db: State<DbPool>,
    Json(new_item): Json<NewItem>,
) -> ApiResult<ApiResponse<201, Json<Item>>> {
    let new_item = Valid::new(new_item)?;
    let mut tx = db.begin().await?;
    let item = item_service::create_item(&mut tx, new_item).await?;
    tx.commit().await?;
    Ok(ApiResponse::created(Json(item)))
}

/// Gets an item.
#[instrument(skip_all, fields(id))]
async fn get_item(
    db: State<DbPool>,
    Path(ItemId { id }): Path<ItemId>,
) -> ApiResult<ApiResponse<200, Json<Item>>> {
    let mut tx = db.begin().await?;
    let item = item_service::read_item(&mut tx, id)
        .await?
        .ok_or(ClientError::NotFound)?;
    tx.commit().await?;
    Ok(ApiResponse::ok(Json(item)))
}

/// Updates an item.
#[instrument(skip(db))]
async fn update_item(
    db: State<DbPool>,
    Path(ItemId { id }): Path<ItemId>,
    Json(new_item): Json<NewItem>,
) -> ApiResult<ApiResponse<200, Json<Item>>> {
    let new_item = Valid::new(new_item)?;
    let mut tx = db.begin().await?;
    let item = item_service::update_item(&mut tx, id, new_item).await?;
    tx.commit().await?;
    Ok(ApiResponse::ok(Json(item)))
}

/// Deletes an item.
#[instrument(skip_all, fields(id))]
async fn delete_item(
    db: State<DbPool>,
    Path(ItemId { id }): Path<ItemId>,
) -> ApiResult<ApiResponse<204, ()>> {
    let mut tx = db.begin().await?;
    item_service::delete_item(&mut tx, id).await?;
    tx.commit().await?;
    Ok(ApiResponse::no_content())
}

/// Lists all items.
#[instrument(skip_all)]
async fn list_items(db: State<DbPool>) -> ApiResult<ApiResponse<200, Json<Vec<Item>>>> {
    let mut tx = db.begin().await?;
    let items = item_service::list_items(&mut tx).await?;
    Ok(ApiResponse::ok(Json(items)))
}

/// Options for how to stream result.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct StreamParams {
    /// The delay between each result in milliseconds.
    throttle: Option<u64>,
}

/// Streams all items.
#[instrument(skip_all, fields(params))]
async fn stream_items<'a>(
    State(db): State<DbPool>,
    Query(params): Query<StreamParams>,
) -> ApiResult<JsonLines<impl Stream<Item = Result<Item, ApiError>>, AsResponse>> {
    let conn = db.acquire().await?;
    let throttle = Duration::from_millis(params.throttle.unwrap_or(0));
    Ok(JsonLines::new(item_service::stream_items(conn, throttle)))
}

#[cfg(test)]
mod tests {}
