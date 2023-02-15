//! A GraphQL api for interacting with items.

use super::GraphQlData;
use crate::core::item::{
    item_repository::{self, ItemRepository},
    item_service,
};
use async_graphql::{Context, Object};

/// A wrapper around an item.
#[derive(Debug)]
pub struct Item(item_repository::Item);

/// An item.
#[Object]
impl Item {
    /// The id of the item.
    async fn id(&self) -> i32 {
        self.0.id
    }

    /// The name of the item.
    async fn name(&self) -> &str {
        &self.0.name
    }

    /// The description of the item.
    async fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }
}

/// The GraphQL API query root.
#[derive(Clone, Copy, Debug)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Finds a specific item.
    async fn item<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "id of the item")] id: i32,
    ) -> Option<Item> {
        let data = ctx.data_unchecked::<GraphQlData>();
        let db = data.db();
        let mut tx = db.begin().await.unwrap();
        let mut item_repository = ItemRepository::new(&mut tx);
        let item = item_service::read_item(&mut item_repository, id)
            .await
            .unwrap();
        item.map(Item)
    }

    /// Lists all items.
    async fn items<'a>(&self, ctx: &Context<'a>) -> Option<Vec<Item>> {
        let data = ctx.data_unchecked::<GraphQlData>();
        let db = data.db();
        let mut tx = db.begin().await.unwrap();
        let mut item_repository = ItemRepository::new(&mut tx);
        let items = item_service::list_items(&mut item_repository)
            .await
            .unwrap();
        Some(items.into_iter().map(Item).collect())
    }
}
