//! Types and functions for storing and loading items from the database.

use crate::infra::{
    database::Tx,
    error::{ApiResult, ClientError},
    security::User,
    validation::Valid,
};
use chrono::{DateTime, Utc};
use http::Uri;
use serde::{Deserialize, Serialize};
use tracing::{instrument, Instrument};
use utoipa::ToSchema;
use validator::Validate;

/// A new URL to shorten.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
pub struct NewUrl {
    /// The name of the shortened URL.
    #[schema(example = "example")]
    #[validate(length(min = 1))]
    pub name: String,
    /// The URL to shorten.
    #[schema(example = "https://example.com")]
    #[serde(with = "http_serde::uri")]
    pub url: Uri,
}

/// An existing shortened URL.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Url {
    /// The URL's id.
    #[schema(example = "1")]
    pub id: i32,
    /// The name of the shortened URL.
    #[schema(example = "example")]
    pub name: String,
    /// The URL to shorten.
    #[schema(example = "https://example.com")]
    pub url: String,
    /// The user who created the URL.
    #[schema(example = "1")]
    pub created_by: i32,
    /// The time the URL was created.
    #[schema(example = "2021-01-01T00:00:00Z")]
    pub created_at: DateTime<Utc>,
}

/// Shortens a new URL.
#[instrument(skip(tx))]
pub async fn create_url<R>(tx: &mut Tx, new_url: Valid<NewUrl>, user: User<R>) -> ApiResult<Url> {
    let new_item = new_url.into_inner();
    tracing::info!("Creating url {:?}", new_item);
    let url = sqlx::query_as!(
        Url,
        r#"
        INSERT INTO urls (name, url, created_by)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        new_item.name,
        new_item.url.to_string(),
        user.id()
    )
    .fetch_one(tx)
    .await?;
    tracing::info!("Created url {:?}", url);
    Ok(url)
}

/// Read a shortened URL.
#[instrument(skip(tx))]
pub async fn fetch_url(tx: &mut Tx, name: &str) -> ApiResult<Option<Url>> {
    tracing::info!("Reading url");
    let item = sqlx::query_as!(
        Url,
        r#"
        SELECT * FROM urls
        WHERE name = $1
        "#,
        name
    )
    .fetch_optional(tx)
    .instrument(tracing::info_span!("fetch_optional"))
    .await?;
    tracing::info!("Found url: {:?}", item);
    Ok(item)
}

/// Deletes a shortened URL.
#[instrument(skip(tx))]
pub async fn delete_url<R>(tx: &mut Tx, name: &str, user: User<R>) -> ApiResult<()> {
    tracing::info!("Deleting url {:?}", name);
    let rows = sqlx::query_as!(
        Item,
        r#"
        DELETE FROM urls
        WHERE name = $1 AND created_by = $2
        "#,
        name,
        user.id()
    )
    .execute(tx)
    .await?;

    if rows.rows_affected() == 0 {
        tracing::warn!("Url not found");
        return Err(ClientError::NotFound)?;
    }

    tracing::info!("Deleted url");

    Ok(())
}

/// Lists all items.
#[instrument(skip(tx))]
pub async fn list_items<R>(tx: &mut Tx, user: User<R>) -> ApiResult<Vec<Url>> {
    tracing::info!("Listing urls");
    let items = sqlx::query_as!(
        Url,
        r#"
        SELECT * FROM urls WHERE created_by = $1
        "#,
        user.id(),
    )
    .fetch_all(tx)
    .instrument(tracing::info_span!("fetch_all"))
    .await?;
    tracing::info!("Listed {} items", items.len());
    Ok(items)
}

#[cfg(test)]
mod tests {}
