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
        error::{ApiError, ApiResult}, security::{User, Admin},
    },
};
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use axum_extra::{json_lines::AsResponse, response::JsonLines};
use futures::Stream;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::IntoParams;

/// The item API endpoints.
pub fn item_routes() -> Router<AppState> {
    Router::new()
        .route("/items", post(create_item))
        .route("/items", get(list_items))
        .route("/items2", get(stream_items))
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

/// Options for how to stream result.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, IntoParams)]
pub struct StreamOptions {
    /// The delay between each result in milliseconds.
    throttle: Option<u64>,
}

/// Streams all items.
#[utoipa::path(
    get,
    path = "/api/items2",
    params(StreamOptions),
    responses(
        (status = 200, description = "Success", body = [Item]),
        (status = 500, description = "Internal error", body = ErrorBody),
    )
)]
#[instrument(skip(db, stream_options))]
pub async fn stream_items<'a>(
    State(db): State<DbPool>,
    Query(stream_options): Query<StreamOptions>,
    user: User<Admin>
) -> ApiResult<JsonLines<impl Stream<Item = Result<Item, ApiError>>, AsResponse>> {
    let conn = db.acquire().await?;
    let throttle = Duration::from_millis(stream_options.throttle.unwrap_or(0));
    Ok(JsonLines::new(item_service::stream_items(conn, throttle)))
}

#[cfg(test)]
mod tests {}