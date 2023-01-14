//! GraphQL API implementation.

use self::graphql_item_api::QueryRoot;
use crate::infra::database::DbPool;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};

pub mod graphql_item_api;

/// The schema
pub type GraphQlSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

/// State for the GraphQL API.
#[derive(Clone, Debug)]
pub struct GraphQlData {
    db: DbPool,
}

impl GraphQlData {
    /// Creates new GraphQL data.
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    /// Returns a reference to the database pool.
    pub fn db(&self) -> &DbPool {
        &self.db
    }
}
