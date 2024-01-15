//! Types and functions for storing and loading items from the database.

use crate::infra::{
    database::{DbConnection, Tx},
    error::{ApiResult, ClientError},
    pagination::PaginationParams,
    validation::Valid,
};
use async_stream::try_stream;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{instrument, Instrument};
use utoipa::ToSchema;
use validator::Validate;

/// A new item.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
pub struct NewItem {
    /// The item's name.
    #[schema(example = "MyItem")]
    #[validate(length(min = 1))]
    pub name: String,
    /// The item's description.
    #[schema(example = "A very interesting item")]
    #[validate(length(min = 1))]
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
pub async fn create_item(tx: &mut Tx, new_item: Valid<NewItem>) -> ApiResult<Item> {
    let new_item = new_item.into_inner();
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
    .fetch_one(tx.as_mut())
    .await?;
    tracing::info!("Created item {:?}", item);
    Ok(item)
}

/// Read an item.
#[instrument(skip(tx))]
pub async fn fetch_item(tx: &mut Tx, id: i32) -> ApiResult<Option<Item>> {
    tracing::info!("Reading item");
    let item = sqlx::query_as!(
        Item,
        r#"
        SELECT * FROM items
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(tx.as_mut())
    .instrument(tracing::info_span!("fetch_optional"))
    .await?;
    tracing::info!("Found item: {:?}", item);
    Ok(item)
}

/// Updates an item.
#[instrument(skip(tx))]
pub async fn update_item(tx: &mut Tx, id: i32, new_item: Valid<NewItem>) -> ApiResult<Item> {
    let new_item = new_item.into_inner();
    tracing::info!("Updating item {:?}", new_item);
    let item = sqlx::query_as!(
        Item,
        r#"
        UPDATE items
        SET name = $1, description = $2
        RETURNING *
        "#,
        new_item.name,
        new_item.description
    )
    .fetch_one(tx.as_mut())
    .await?;
    tracing::info!("Updated item {:?}", item);
    Ok(item)
}

/// Deletes an item.
#[instrument(skip(tx))]
pub async fn delete_item(tx: &mut Tx, id: i32) -> ApiResult<()> {
    tracing::info!("Deleting item {:?}", id);
    let rows = sqlx::query_as!(
        Item,
        r#"
        DELETE FROM items
        WHERE id = $1
        "#,
        id
    )
    .execute(tx.as_mut())
    .await?;

    if rows.rows_affected() == 0 {
        tracing::warn!("Item not found");
        return Err(ClientError::NotFound)?;
    }

    tracing::info!("Deleted item");

    Ok(())
}

/// Lists all items.
#[instrument(skip(tx))]
pub async fn list_items(tx: &mut Tx, params: &PaginationParams) -> ApiResult<Vec<Item>> {
    tracing::info!("Listing items");
    let items = sqlx::query_as!(
        Item,
        r#"
        SELECT * FROM items
        LIMIT $1
        OFFSET $2
        "#,
        params.limit(),
        params.offset()
    )
    .fetch_all(tx.as_mut())
    .instrument(tracing::info_span!("fetch_all"))
    .await?;
    tracing::info!("Listed {} items", items.len());
    Ok(items)
}

/// Streams all items.
#[allow(clippy::let_with_type_underscore)]
#[instrument(skip(conn))]
pub fn stream_items(
    mut conn: DbConnection,
    params: PaginationParams,
    throttle: Duration,
) -> impl Stream<Item = ApiResult<Item>> {
    tracing::info!("Streaming items");
    let items = try_stream! {
        let mut items = sqlx::query_as!(
            Item,
            r#"
                SELECT * FROM items
                LIMIT $1
                OFFSET $2
            "#,
            params.limit(),
            params.offset()
        )
        .fetch(conn.as_mut());
        let mut total = 0;
        while let Some(item) = items.next().await {
            yield item?;
            total += 1;
            tokio::time::sleep(throttle).await;
        }
        tracing::info!("Streamed {} items", total);
    };
    Box::pin(items)
}

#[cfg(test)]
mod tests {
    use crate::infra::pagination::PaginationParams;

    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_then_list_returns_item(db: PgPool) {
        let mut tx = db.begin().await.unwrap();
        let item = create_item(
            &mut tx,
            Valid::new(NewItem {
                name: "Foo".to_string(),
                description: None,
            })
            .unwrap(),
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

        let items = list_items(&mut tx, &PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(&item, items.last().unwrap());
    }
}
