use axum::{
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::instrument;

pub fn item_routes() -> Router {
    Router::new()
        .route("/items", post(create_item))
        .route("/items", get(list_items))
}

/// A name query parameter.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewItem {
    name: String,
    description: Option<String>,
}

/// This is a response to the hello endpoint.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    id: i32,
    name: String,
    description: Option<String>,
}

/// Creates a new item.
#[instrument]
pub async fn create_item(
    Extension(db): Extension<PgPool>,
    Json(item): Json<NewItem>,
) -> Json<Item> {
    let mut tx = db.begin().await.unwrap();
    let item = sqlx::query_as!(
        Item,
        r#"
        INSERT INTO items (name, description)
        VALUES ($1, $2)
        RETURNING *
        "#,
        item.name,
        item.description
    )
    .fetch_one(&mut tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    Json(item)
}

/// Lists all items.
#[instrument]
pub async fn list_items(Extension(db): Extension<PgPool>) -> Json<Vec<Item>> {
    let mut tx = db.begin().await.unwrap();
    let items = sqlx::query_as!(
        Item,
        r#"
        SELECT * FROM items
        "#
    )
    .fetch_all(&mut tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    Json(items)
}

#[cfg(test)]
mod tests {
    use super::{create_item, Item};
    use crate::rest::items::{list_items, NewItem};
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
