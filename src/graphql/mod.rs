//! GraphQL API implementation.

use self::graphql_item_api::QueryRoot;
use crate::infra::database::DbPool;
use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    response::{Html, IntoResponse},
    Extension,
};

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

/// A handler for GraphQL requests.
pub async fn graphql_handler(
    schema: Extension<GraphQlSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// A handler for the GraphQL IDE.
pub async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphiql").finish())
}
