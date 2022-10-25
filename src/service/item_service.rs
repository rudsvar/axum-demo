//! A service for interacting with items.

use crate::{
    infra::{database::Tx, error::ApiResult},
    repository::item_repository::{self, Item, NewItem},
};
use tracing::instrument;

/// Creates a new item.
#[instrument(skip(tx))]
pub async fn create_item(tx: &mut Tx, new_item: NewItem) -> ApiResult<Item> {
    let item = item_repository::create_item(tx, new_item).await?;
    Ok(item)
}

/// Lists all items.
#[instrument(skip(tx))]
pub async fn list_items(tx: &mut Tx) -> ApiResult<Vec<Item>> {
    let items = item_repository::list_items(tx).await?;
    Ok(items)
}
