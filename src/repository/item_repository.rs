use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use tracing::instrument;

/// A new item.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewItem {
    pub name: String,
    pub description: Option<String>,
}

/// An existing item.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

/// Creates a new item.
#[instrument]
pub async fn create_item(tx: &mut Transaction<'static, Postgres>, new_item: NewItem) -> Item {
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
    .await
    .unwrap();
    item
}

/// Lists all items.
#[instrument]
pub async fn list_items(tx: &mut Transaction<'static, Postgres>) -> Vec<Item> {
    let items = sqlx::query_as!(
        Item,
        r#"
        SELECT * FROM items
        "#
    )
    .fetch_all(tx)
    .await
    .unwrap();
    items
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
        .await;

        assert_eq!(
            Item {
                id: 1,
                name: "Foo".to_string(),
                description: None,
            },
            item,
        );

        let items = list_items(&mut tx).await;
        assert_eq!(&item, items.last().unwrap());
    }
}
