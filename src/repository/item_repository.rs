//! Types and functions for storing and loading items from the database.

use crate::infra::{database::Tx, error::ApiResult};
use serde::{Deserialize, Serialize};
use tracing::{instrument, Instrument};
use utoipa::ToSchema;

/// A new item.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewItem {
    /// The item's name.
    #[schema(example = "MyItem")]
    pub name: String,
    /// The item's description.
    #[schema(example = "A very interesting item")]
    pub description: Option<String>,
}

/// An existing item.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Item {
    /// The item's id.
    pub id: i32,
    #[schema(example = "MyItem")]
    /// The item's name.
    pub name: String,
    #[schema(example = "A very interesting item")]
    /// The item's description.
    pub description: Option<String>,
}

/// Creates a new item.
#[instrument(skip(tx))]
pub async fn create_item(tx: &mut Tx, new_item: NewItem) -> ApiResult<Item> {
    tracing::info!("Creating item {:?}", new_item);
    let item = sqlx::query_as!(
        Item,
        r#"
        INSERT INTO items (name, description)
        VALUES ($1, $2)
        RETURNING *
        "#,
        new_item.name,
        new_item.description
    )
    .fetch_one(tx)
    .await?;
    tracing::info!("Created item {:?}", item);
    Ok(item)
}

/// Lists all items.
#[instrument(skip(tx))]
pub async fn list_items(tx: &mut Tx) -> ApiResult<Vec<Item>> {
    tracing::info!("Listing items");
    let items = sqlx::query_as!(
        Item,
        r#"
        SELECT * FROM items
        "#
    )
    .fetch_all(tx)
    .instrument(tracing::info_span!("fetch_all"))
    .await?;
    tracing::info!("Got items {:?}", items);
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::{create_item, list_items, Item};
    use crate::repository::item_repository::NewItem;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_then_list_returns_item(db: PgPool) {
        let mut tx = db.begin().await.unwrap();
        let item = create_item(
            &mut tx,
            NewItem {
                name: "Foo".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            Item {
                id: 1,
                name: "Foo".to_string(),
                description: None,
            },
            item,
        );

        let items = list_items(&mut tx).await.unwrap();
        assert_eq!(&item, items.last().unwrap());
    }
}
