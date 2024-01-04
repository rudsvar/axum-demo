//! Types and functions for storing and loading items from the database.

use crate::infra::{
    database::Tx,
    error::{ApiResult, ClientError},
    security::User,
    validation::Valid,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{instrument, Instrument};
use utoipa::ToSchema;
use validator::Validate;

/// A new URL to shorten.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
pub struct NewShortUrl {
    /// The name of the shortened URL.
    #[schema(example = "example")]
    #[validate(length(min = 1))]
    pub name: String,
    /// The URL to shorten.
    #[schema(example = "https://example.com")]
    #[validate(url)]
    pub target: String,
}

/// An existing shortened URL.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ShortUrl {
    /// The URL's id.
    #[schema(example = "1")]
    pub id: i32,
    /// The name of the shortened URL.
    #[schema(example = "example")]
    pub name: String,
    /// The URL to redirect to.
    #[schema(example = "https://example.com")]
    pub target: String,
    /// The user who created the URL.
    #[schema(example = "1")]
    pub created_by: i32,
    /// The time the URL was created.
    #[schema(example = "2021-01-01T00:00:00Z")]
    pub created_at: DateTime<Utc>,
}

/// Shortens a new URL.
#[instrument(skip(tx))]
pub async fn create_url<R>(
    tx: &mut Tx,
    new_url: Valid<NewShortUrl>,
    user: User<R>,
) -> ApiResult<ShortUrl> {
    let new_item = new_url.into_inner();
    tracing::info!("Creating url {:?}", new_item);
    let url = sqlx::query_as!(
        ShortUrl,
        r#"
        INSERT INTO short_urls (name, target, created_by)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        new_item.name,
        new_item.target.to_string(),
        user.id()
    )
    .fetch_one(tx.as_mut())
    .await?;
    tracing::info!("Created url {:?}", url);
    Ok(url)
}

/// Read a shortened URL.
#[instrument(skip(tx))]
pub async fn fetch_url(tx: &mut Tx, name: &str) -> ApiResult<Option<ShortUrl>> {
    tracing::info!("Reading url");
    let item = sqlx::query_as!(
        ShortUrl,
        r#"
        SELECT * FROM short_urls
        WHERE name = $1
        "#,
        name
    )
    .fetch_optional(tx.as_mut())
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
        DELETE FROM short_urls
        WHERE name = $1 AND created_by = $2
        "#,
        name,
        user.id()
    )
    .execute(tx.as_mut())
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
pub async fn list_items<R>(tx: &mut Tx, user: User<R>) -> ApiResult<Vec<ShortUrl>> {
    tracing::info!("Listing urls");
    let items = sqlx::query_as!(
        ShortUrl,
        r#"
        SELECT * FROM short_urls WHERE created_by = $1
        "#,
        user.id(),
    )
    .fetch_all(tx.as_mut())
    .instrument(tracing::info_span!("fetch_all"))
    .await?;
    tracing::info!("Listed {} items", items.len());
    Ok(items)
}

#[cfg(test)]
mod tests {}
