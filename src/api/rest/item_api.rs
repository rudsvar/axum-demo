use axum::{
    routing::{get, post},
    Extension, Json, Router,
};
use sqlx::PgPool;
use tracing::instrument;
use crate::{
    repository::item_repository::{Item, NewItem},
    service::item_service,
};

pub fn item_routes() -> Router {
    Router::new()
        .route("/items", post(create_item))
        .route("/items", get(list_items))
}

/// Creates a new item.
#[instrument]
pub async fn create_item(
    Extension(db): Extension<PgPool>,
    Json(new_item): Json<NewItem>,
) -> Json<Item> {
    let item = item_service::create_item(db, new_item).await;
    Json(item)
}

/// Lists all items.
#[instrument]
pub async fn list_items(Extension(db): Extension<PgPool>) -> Json<Vec<Item>> {
    let items = item_service::list_items(db).await;
    Json(items)
}

#[cfg(test)]
mod tests {
    use super::{create_item, Item};
    use crate::api::rest::item_api::{list_items, NewItem};
    use axum::{Extension, Json};
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_then_list_returns_item(db: PgPool) {
        let item = create_item(
            Extension(db.clone()),
            Json(NewItem {
                name: "Foo".to_string(),
                description: None,
            }),
        )
        .await;

        assert_eq!(
            Item {
                id: 1,
                name: "Foo".to_string(),
                description: None,
            },
            item.0,
        );

        let items = list_items(Extension(db.clone())).await;
        assert_eq!(&item.0, items.last().unwrap());
    }
}
