//! Implementation of a gRPC item service.

use self::item::{
    item_service_server::ItemService, CreateItemRequest, CreateItemResponse, ListItemsRequest,
    ListItemsResponse,
};
use crate::{
    core::item::item_repository::{self, NewItem},
    infra::{
        database::DbPool,
        error::{ApiError, ClientError},
    },
};
use tonic::Status;

/// Generated traits and types for the item gRPC API.
#[allow(clippy::derive_partial_eq_without_eq, clippy::module_inception)]
pub(super) mod item {
    tonic::include_proto!("item");
}

/// An struct that should implement [`MyGreeter`].
#[derive(Clone, Debug)]
pub struct ItemServiceImpl {
    db: DbPool,
}

impl ItemServiceImpl {
    /// Creates a new instance of the service that connects to the specified database.
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl ItemService for ItemServiceImpl {
    async fn create_item(
        &self,
        request: tonic::Request<CreateItemRequest>,
    ) -> Result<tonic::Response<CreateItemResponse>, Status> {
        // Create transaction
        let mut tx = self.db.begin().await.map_err(ApiError::from)?;
        // Map request type to item
        let request: CreateItemRequest = request.into_inner();
        let new_item = request
            .item
            .ok_or_else(|| ClientError::BadRequest("missing item".to_string()))
            .map_err(ApiError::from)?;
        let new_item = NewItem {
            name: new_item.name,
            description: Some(new_item.description),
        };
        // Create item
        let item = item_repository::create_item(&mut tx, new_item).await?;
        // Map item to response type
        let item = self::item::Item {
            id: item.id,
            name: item.name,
            description: item.description.unwrap_or_default(),
        };
        let response = CreateItemResponse { item: Some(item) };

        // Commit and respond
        tx.commit().await.map_err(ApiError::from)?;
        Ok(tonic::Response::new(response))
    }

    async fn list_items(
        &self,
        _: tonic::Request<ListItemsRequest>,
    ) -> Result<tonic::Response<ListItemsResponse>, Status> {
        // Create transaction
        let mut tx = self.db.begin().await.map_err(ApiError::from)?;
        // List items
        let items = item_repository::list_items(&mut tx).await?;
        let items: Vec<_> = items
            .into_iter()
            .map(|item| item::Item {
                id: item.id,
                name: item.name,
                description: item.description.unwrap_or_default(),
            })
            .collect();
        // Map item to response type
        let response = ListItemsResponse { items };
        Ok(tonic::Response::new(response))
    }
}

#[cfg(test)]
mod tests {}
