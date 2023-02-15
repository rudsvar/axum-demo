//! A service for interacting with items.

use super::item_repository::{CreateItem, FetchItem, ListItems, StreamItems};
use crate::{
    core::item::item_repository::{Item, NewItem},
    infra::error::ApiResult,
};
use futures::Stream;
use std::time::Duration;
use tracing::instrument;

/// Creates a new item.
#[instrument(skip(repository))]
pub async fn create_item<R: CreateItem>(repository: &mut R, new_item: NewItem) -> ApiResult<Item> {
    repository.create_item(new_item).await
}

/// Read an item.
#[instrument(skip(repository))]
pub async fn read_item<R: FetchItem>(repository: &mut R, id: i32) -> ApiResult<Option<Item>> {
    repository.fetch_item(id).await
}

/// Lists all items.
#[instrument(skip(repository))]
pub async fn list_items<R: ListItems>(repository: &mut R) -> ApiResult<Vec<Item>> {
    repository.list_items().await
}

/// Streams all items.
#[instrument(skip(repository))]
pub fn stream_items<R: StreamItems>(
    repository: R,
    throttle: Duration,
) -> impl Stream<Item = ApiResult<Item>> {
    repository.stream_items(throttle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::item::item_repository::MockCreateItem;

    #[tokio::test]
    async fn create_item() {
        let mut repo = MockCreateItem::new();
        let expected_item = Item {
            id: 1,
            name: "Foo".to_string(),
            description: None,
        };
        let expected_item_clone = expected_item.clone();
        repo.expect_create_item()
            .return_once(move |_| Ok(expected_item));

        let item = repo
            .create_item(NewItem {
                name: "Foo".to_string(),
                description: None,
            })
            .await
            .unwrap();

        assert_eq!(expected_item_clone, item,);
    }
}
