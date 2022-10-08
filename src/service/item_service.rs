use sqlx::PgPool;
use tracing::instrument;

use crate::repository::item_repository::{self, Item, NewItem};

/// Creates a new item.
#[instrument(skip(db))]
pub async fn create_item(db: PgPool, new_item: NewItem) -> Item {
    let mut tx = db.begin().await.unwrap();
    let item = item_repository::create_item(&mut tx, new_item).await;
    tx.commit().await.unwrap();
    item
}

/// Lists all items.
#[instrument(skip(db))]
pub async fn list_items(db: PgPool) -> Vec<Item> {
    let mut tx = db.begin().await.unwrap();
    let items = item_repository::list_items(&mut tx).await;
    tx.commit().await.unwrap();
    items
}
