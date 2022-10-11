use sqlx::PgPool;
use tracing::{instrument, Instrument};

use crate::{
    infra::error::ApiResult,
    repository::item_repository::{self, Item, NewItem},
};

/// Creates a new item.
#[instrument(skip(db))]
pub async fn create_item(db: PgPool, new_item: NewItem) -> ApiResult<Item> {
    let mut tx = db.begin().await?;
    let item = item_repository::create_item(&mut tx, new_item).await?;
    tx.commit().await?;
    Ok(item)
}

/// Lists all items.
#[instrument(skip(db))]
pub async fn list_items(db: PgPool) -> ApiResult<Vec<Item>> {
    let mut tx = db
        .acquire()
        .instrument(tracing::info_span!("acquire"))
        .await?;
    let items = item_repository::list_items(&mut tx).await;
    items
}
