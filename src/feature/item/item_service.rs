//! A service for interacting with items.

use crate::{
    feature::item::item_repository::{self, Item, NewItem},
    infra::{
        database::{DbConnection, Tx},
        error::ApiResult,
        pagination::PaginationParams,
        validation::Valid,
    },
};
use futures::Stream;
use std::time::Duration;
use tracing::instrument;

/// Creates a new item.
#[instrument(skip(tx))]
pub async fn create_item(tx: &mut Tx, new_item: Valid<NewItem>) -> ApiResult<Item> {
    item_repository::create_item(tx, new_item).await
}

/// Updates an item.
#[instrument(skip(tx))]
pub async fn update_item(tx: &mut Tx, id: i32, new_item: Valid<NewItem>) -> ApiResult<Item> {
    item_repository::update_item(tx, id, new_item).await
}

/// Read an item.
#[instrument(skip(tx))]
pub async fn read_item(tx: &mut Tx, id: i32) -> ApiResult<Option<Item>> {
    item_repository::fetch_item(tx, id).await
}

/// Delete an item.
#[instrument(skip(tx))]
pub async fn delete_item(tx: &mut Tx, id: i32) -> ApiResult<()> {
    item_repository::delete_item(tx, id).await
}

/// Lists all items.
#[instrument(skip(tx))]
pub async fn list_items(tx: &mut Tx, params: &PaginationParams) -> ApiResult<Vec<Item>> {
    item_repository::list_items(tx, params).await
}

/// Streams all items.
#[allow(clippy::let_with_type_underscore)]
#[instrument(skip(conn))]
pub fn stream_items(
    conn: DbConnection,
    params: PaginationParams,
    throttle: Duration,
) -> impl Stream<Item = ApiResult<Item>> {
    item_repository::stream_items(conn, params, throttle)
}
