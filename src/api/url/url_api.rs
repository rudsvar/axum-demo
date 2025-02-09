//! The url API implementation.

use crate::infra::{
    database::DbPool,
    error::{ApiResult, ClientError, ErrorBody},
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
#[typed_path("/urls/{id}", rejection(ClientError))]
struct UrlsId(String);

/// Shortens a new URL.
#[utoipa::path(
    post,
    path = "/api/urls",
    request_body = NewShortUrl,
    responses(
        (status = 201, description = "Created", body = ShortUrl),
        (status = 409, description = "Conflict", body = ErrorBody),
        (status = 500, description = "Internal Server Error", body = ErrorBody),
    ),
    security(
        ("basic" = [])
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/urls/{name}",
    responses(
        (status = 303, description = "See Other", body = ShortUrl),
        (status = 404, description = "Not Found", body = ErrorBody),
        (status = 500, description = "Internal Server Error", body = ErrorBody),
    )
)]
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
#[utoipa::path(
    delete,
    path = "/api/urls/{id}",
    responses(
        (status = 200, description = "Ok", body = ShortUrl),
        (status = 404, description = "Not Found", body = ErrorBody),
        (status = 500, description = "Internal Server Error", body = ErrorBody),
    ),
    security(
        ("basic" = [])
    )
)]
#[instrument(skip_all, fields(id))]
async fn delete_url(UrlsId(id): UrlsId, db: State<DbPool>, user: User) -> ApiResult<StatusCode> {
    let mut tx = db.begin().await?;
    url_repository::delete_url(&mut tx, &id, user).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Lists all shortened URLs.
#[utoipa::path(
    get,
    path = "/api/urls",
    responses(
        (status = 200, description = "Success", body = [ShortUrl]),
        (status = 500, description = "Internal error", body = ErrorBody),
    ),
    security(
        ("basic" = [])
    )
)]
#[instrument(skip_all)]
async fn list_urls(Urls: Urls, db: State<DbPool>, user: User) -> ApiResult<Json<Vec<ShortUrl>>> {
    let mut tx = db.begin().await?;
    let urls = url_repository::list_urls(&mut tx, user).await?;
    Ok(Json(urls))
}

#[cfg(test)]
mod tests {}
