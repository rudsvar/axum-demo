//! The url API implementation.

use crate::infra::{
    database::DbPool,
    error::{ApiResult, ClientError},
    extract::Json,
    security::User,
    state::AppState,
    validation::Valid,
};
use axum::{extract::State, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use serde::Deserialize;
use tracing::instrument;

use super::url_repository::{self, NewShortUrl, ShortUrl};

/// The url API endpoints.
pub fn routes() -> Router<AppState> {
    Router::new()
        .typed_post(create_url)
        .typed_get(visit_url)
        .typed_delete(delete_url)
        .typed_get(list_urls)
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/urls", rejection(ClientError))]
struct Urls;

#[derive(Deserialize, TypedPath)]
#[typed_path("/urls/:id", rejection(ClientError))]
struct UrlsId(String);

/// Shortens a new URL.
#[instrument(skip_all, fields(new_item))]
async fn create_url(
    Urls: Urls,
    db: State<DbPool>,
    user: User,
    Json(new_url): Json<NewShortUrl>,
) -> ApiResult<(StatusCode, Json<ShortUrl>)> {
    let new_url = Valid::new(new_url)?;
    let mut tx = db.begin().await?;
    let url = url_repository::create_url(&mut tx, new_url, user).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(url)))
}

/// Gets a shortened URL.
#[instrument(skip_all, fields(id))]
async fn visit_url(
    UrlsId(name): UrlsId,
    db: State<DbPool>,
) -> ApiResult<(StatusCode, HeaderMap, Json<ShortUrl>)> {
    let mut tx = db.begin().await?;
    let url = url_repository::fetch_url(&mut tx, &name)
        .await?
        .ok_or(ClientError::NotFound)?;
    tx.commit().await?;
    let mut hm = HeaderMap::new();
    hm.append(
        HeaderName::from_static("location"),
        HeaderValue::from_str(&url.target).expect("invalid url"),
    );
    Ok((StatusCode::SEE_OTHER, hm, Json(url)))
}

/// Deletes a shortened URL.
#[instrument(skip_all, fields(id))]
async fn delete_url(UrlsId(id): UrlsId, db: State<DbPool>, user: User) -> ApiResult<StatusCode> {
    let mut tx = db.begin().await?;
    url_repository::delete_url(&mut tx, &id, user).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Lists all shortened URLs.
#[instrument(skip_all)]
async fn list_urls(Urls: Urls, db: State<DbPool>, user: User) -> ApiResult<Json<Vec<ShortUrl>>> {
    let mut tx = db.begin().await?;
    let urls = url_repository::list_items(&mut tx, user).await?;
    Ok(Json(urls))
}

#[cfg(test)]
mod tests {}
