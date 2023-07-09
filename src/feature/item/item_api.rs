//! The item API implementation.

use crate::{
    feature::item::{
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
use axum::{extract::State, Router};
use axum_extra::{
    json_lines::AsResponse,
    response::JsonLines,
    routing::{RouterExt, TypedPath},
};
use futures::Stream;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;
use utoipa::IntoParams;

/// The item API endpoints.
pub fn routes() -> Router<AppState> {
    Router::new()
        .typed_post(create_item)
        .typed_get(get_item)
        .typed_put(update_item)
        .typed_delete(delete_item)
        .typed_get(list_items)
        .typed_get(stream_items)
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/items", rejection(ClientError))]
struct Items;

#[derive(Deserialize, TypedPath)]
#[typed_path("/items2", rejection(ClientError))]
struct Items2;

#[derive(Deserialize, TypedPath)]
#[typed_path("/items/:id", rejection(ClientError))]
struct ItemsId(i32);

/// Creates a new item.
#[utoipa::path(
    post,
    path = "/api/items",
    request_body = NewItem,
    responses(
        (status = 201, description = "Created", body = Item),
        (status = 409, description = "Conflict", body = ErrorBody),
        (status = 500, description = "Internal Server Error", body = ErrorBody),
    )
)]
#[instrument(skip_all, fields(new_item))]
async fn create_item(
    Items: Items,
    db: State<DbPool>,
    Json(new_item): Json<NewItem>,
) -> ApiResult<(StatusCode, Json<Item>)> {
    let new_item = Valid::new(new_item)?;
    let mut tx = db.begin().await?;
    let item = item_service::create_item(&mut tx, new_item).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(item)))
}

/// Gets an item.
#[utoipa::path(
    get,
    path = "/api/items/{id}",
    responses(
        (status = 200, description = "Ok", body = Item),
        (status = 404, description = "Not Found", body = ErrorBody),
        (status = 500, description = "Internal Server Error", body = ErrorBody),
    )
)]
#[instrument(skip_all, fields(id))]
async fn get_item(ItemsId(id): ItemsId, db: State<DbPool>) -> ApiResult<(StatusCode, Json<Item>)> {
    let mut tx = db.begin().await?;
    let item = item_service::read_item(&mut tx, id)
        .await?
        .ok_or(ClientError::NotFound)?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(item)))
}

/// Updates an item.
#[utoipa::path(
    put,
    path = "/api/items/{id}",
    request_body = NewItem,
    responses(
        (status = 200, description = "Ok", body = Item),
        (status = 404, description = "Not Found", body = ErrorBody),
        (status = 500, description = "Internal Server Error", body = ErrorBody),
    )
)]
#[instrument(skip(db))]
async fn update_item(
    ItemsId(id): ItemsId,
    db: State<DbPool>,
    Json(new_item): Json<NewItem>,
) -> ApiResult<(StatusCode, Json<Item>)> {
    let new_item = Valid::new(new_item)?;
    let mut tx = db.begin().await?;
    let item = item_service::update_item(&mut tx, id, new_item).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(item)))
}

/// Deletes an item.
#[utoipa::path(
    delete,
    path = "/api/items/{id}",
    responses(
        (status = 200, description = "Ok", body = Item),
        (status = 404, description = "Not Found", body = ErrorBody),
        (status = 500, description = "Internal Server Error", body = ErrorBody),
    )
)]
#[instrument(skip_all, fields(id))]
async fn delete_item(ItemsId(id): ItemsId, db: State<DbPool>) -> ApiResult<StatusCode> {
    let mut tx = db.begin().await?;
    item_service::delete_item(&mut tx, id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
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
#[instrument(skip_all)]
async fn list_items(Items: Items, db: State<DbPool>) -> ApiResult<Json<Vec<Item>>> {
    let mut tx = db.begin().await?;
    let items = item_service::list_items(&mut tx).await?;
    Ok(Json(items))
}

/// Options for how to stream result.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, IntoParams)]
pub struct StreamParams {
    /// The delay between each result in milliseconds.
    throttle: Option<u64>,
}

/// Streams all items.
#[utoipa::path(
    get,
    path = "/api/items2",
    params(StreamParams),
    responses(
        (status = 200, description = "Success", body = [Item]),
        (status = 500, description = "Internal error", body = ErrorBody),
    )
)]
#[instrument(skip_all, fields(params))]
async fn stream_items<'a>(
    Items2: Items2,
    State(db): State<DbPool>,
    Query(params): Query<StreamParams>,
) -> ApiResult<JsonLines<impl Stream<Item = Result<Item, ApiError>>, AsResponse>> {
    let conn = db.acquire().await?;
    let throttle = Duration::from_millis(params.throttle.unwrap_or(0));
    Ok(JsonLines::new(item_service::stream_items(conn, throttle)))
}

#[cfg(test)]
mod tests {}
