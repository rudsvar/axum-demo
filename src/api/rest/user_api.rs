use crate::repository::user_repository;
use axum::{
    headers::{authorization::Basic, Authorization},
    routing::post,
    Extension, Json, Router, TypedHeader,
};
use hyper::StatusCode;
use sqlx::PgPool;
use tracing::instrument;

pub fn user_routes() -> Router {
    Router::new().route("/login", post(login))
}

/// Creates a new item.
#[instrument(skip(db, basic_auth))]
pub async fn login(
    Extension(db): Extension<PgPool>,
    TypedHeader(basic_auth): TypedHeader<Authorization<Basic>>,
) -> Result<Json<i32>, StatusCode> {
    tracing::info!("Fetching connection");
    let mut tx = db.acquire().await.unwrap();
    let username = basic_auth.username();
    let password = basic_auth.password();
    tracing::info!("Authenticating user");
    let id = user_repository::authenticate(&mut tx, username, password).await;
    tracing::info!("Returning");
    id.map(Json).ok_or(StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use axum::{headers::Authorization, Extension, Json, TypedHeader};
    use hyper::StatusCode;
    use sqlx::PgPool;

    use crate::api::rest::user_api::login;

    #[sqlx::test]
    async fn user_with_correct_password_can_login(db: PgPool) {
        let username = "user";
        let password = "user";
        let basic_auth = TypedHeader(Authorization::basic(username, password));
        let Json(id) = login(Extension(db), basic_auth).await.unwrap();
        assert_eq!(1, id);
    }

    #[sqlx::test]
    async fn user_with_wrong_password_cannot_login(db: PgPool) {
        let username = "user";
        let password = "notuser";
        let basic_auth = TypedHeader(Authorization::basic(username, password));
        let status_code = login(Extension(db), basic_auth).await.unwrap_err();
        assert_eq!(StatusCode::UNAUTHORIZED, status_code);
    }
}
