//! The item API implementation.

use std::time::Duration;

use super::AppState;
use crate::{
    core::item::{
        item_repository::{Item, NewItem},
        item_service,
    },
    infra::{
        database::DbPool,
        error::{ApiError, ApiResult, ClientError},
        extract::{Json, Query},
        validation::Valid,
    },
};
use axum::{
    debug_handler,
    extract::{Path, State},
    routing::{delete, get, post, put},
    Router,
};
use axum_extra::{json_lines::AsResponse, response::JsonLines};
use futures::Stream;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::IntoParams;

/// The item API endpoints.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/items", post(create_item))
        .route("/items/:id", get(get_item))
        .route("/items/:id", put(update_item))
        .route("/items/:id", delete(delete_item))
        .route("/items", get(list_items))
        .route("/items2", get(stream_items))
}

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
#[instrument(skip(db))]
#[debug_handler]
async fn create_item(
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
#[instrument(skip(db))]
async fn get_item(db: State<DbPool>, Path(id): Path<i32>) -> ApiResult<(StatusCode, Json<Item>)> {
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
    db: State<DbPool>,
    Path(id): Path<i32>,
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
#[instrument(skip(db))]
async fn delete_item(db: State<DbPool>, Path(id): Path<i32>) -> ApiResult<StatusCode> {
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
#[instrument(skip(db))]
pub async fn list_items(db: State<DbPool>) -> ApiResult<Json<Vec<Item>>> {
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
#[instrument(skip(db, params))]
pub async fn stream_items<'a>(
    State(db): State<DbPool>,
    Query(params): Query<StreamParams>,
) -> ApiResult<JsonLines<impl Stream<Item = Result<Item, ApiError>>, AsResponse>> {
    let conn = db.acquire().await?;
    let throttle = Duration::from_millis(params.throttle.unwrap_or(0));
    Ok(JsonLines::new(item_service::stream_items(conn, throttle)))
}

#[cfg(test)]
mod tests {}
