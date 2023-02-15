//! Types and functions for storing and loading items from the database.

use crate::infra::{
    database::{DbConnection, Tx},
    error::ApiResult,
};
use async_stream::try_stream;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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

/// Anything that can create an item.
#[mockall::automock]
#[async_trait::async_trait]
pub trait CreateItem {
    /// Creates a new item.
    async fn create_item(&mut self, item: NewItem) -> ApiResult<Item>;
}

/// Anything that can fetch an item.
#[mockall::automock]
#[async_trait::async_trait]
pub trait FetchItem {
    /// Fetches an item.
    async fn fetch_item(&mut self, id: i32) -> ApiResult<Option<Item>>;
}

/// Anything that can list items.
#[mockall::automock]
#[async_trait::async_trait]
pub trait ListItems {
    /// Lists items.
    async fn list_items(&mut self) -> ApiResult<Vec<Item>>;
}

/// Anything that can stream items.
#[mockall::automock]
#[async_trait::async_trait]
pub trait StreamItems {
    /// Streams all items.
    fn stream_items(self, throttle: Duration) -> impl Stream<Item = ApiResult<Item>>;
}

/// An item repository.
#[derive(Debug)]
pub struct ItemRepository<E> {
    executor: E,
}

impl<E> ItemRepository<E> {
    /// Creates a new repository.
    pub fn new(executor: E) -> Self {
        Self { executor }
    }
}

#[async_trait::async_trait]
impl CreateItem for ItemRepository<&mut Tx> {
    #[instrument(skip(self))]
    async fn create_item(&mut self, new_item: NewItem) -> ApiResult<Item> {
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
        .fetch_one(&mut *self.executor)
        .await?;
        tracing::info!("Created item {:?}", item);
        Ok(item)
    }
}

#[async_trait::async_trait]
impl FetchItem for ItemRepository<&mut Tx> {
    #[instrument(skip(self))]
    async fn fetch_item(&mut self, id: i32) -> ApiResult<Option<Item>> {
        tracing::info!("Reading item");
        let item = sqlx::query_as!(
            Item,
            r#"
                SELECT * FROM items
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&mut *self.executor)
        .instrument(tracing::info_span!("fetch_optional"))
        .await?;
        tracing::info!("Found item: {:?}", item);
        Ok(item)
    }
}

#[async_trait::async_trait]
impl ListItems for ItemRepository<&mut Tx> {
    #[instrument(skip(self))]
    async fn list_items(&mut self) -> ApiResult<Vec<Item>> {
        tracing::info!("Listing items");
        let items = sqlx::query_as!(
            Item,
            r#"
                SELECT * FROM items
            "#
        )
        .fetch_all(&mut *self.executor)
        .instrument(tracing::info_span!("fetch_all"))
        .await?;
        tracing::info!("Listed {} items", items.len());
        Ok(items)
    }
}

#[async_trait::async_trait]
impl StreamItems for ItemRepository<DbConnection> {
    #[instrument(skip(self))]
    fn stream_items(mut self, throttle: Duration) -> impl Stream<Item = ApiResult<Item>> {
        tracing::info!("Streaming items");
        let items = try_stream! {
            let mut items = sqlx::query_as!(Item, r#"SELECT * FROM items"#).fetch(&mut self.executor);
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
}
