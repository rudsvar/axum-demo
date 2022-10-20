use crate::{
    infra::error::ApiResult,
    repository::item_repository::{Item, NewItem},
    service::item_service,
};
use axum::{Extension, Json, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::instrument;

pub fn item_routes() -> Router {
    Router::new().typed_post(create_item).typed_get(list_items)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/items")]
pub struct ItemsPath;

/// Creates a new item.
#[instrument(skip(db))]
async fn create_item(
    _: ItemsPath,
    Extension(db): Extension<PgPool>,
    Json(new_item): Json<NewItem>,
) -> ApiResult<Json<Item>> {
    let mut tx = db.begin().await?;
    let item = item_service::create_item(&mut tx, new_item).await?;
    tx.commit().await?;
    Ok(Json(item))
}

/// Lists all items.
#[instrument(skip(db))]
pub async fn list_items(
    _: ItemsPath,
    Extension(db): Extension<PgPool>,
) -> ApiResult<Json<Vec<Item>>> {
    let mut tx = db.begin().await?;
    let items = item_service::list_items(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(items))
}

#[cfg(test)]
mod tests {
    use super::{create_item, Item};
    use crate::api::rest::item_api::{list_items, ItemsPath, NewItem};
    use axum::{Extension, Json};
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_then_list_returns_item(db: PgPool) {
        let item = create_item(
            ItemsPath,
            Extension(db.clone()),
            Json(NewItem {
                name: "Foo".to_string(),
                description: None,
            }),
        )
        .await
        .unwrap();

        assert_eq!(
            Item {
                id: 1,
                name: "Foo".to_string(),
                description: None,
            },
            item.0,
        );

        let items = list_items(ItemsPath, Extension(db.clone())).await.unwrap();
        assert_eq!(&item.0, items.last().unwrap());
    }
}
