use crate::{
    infra::error::{ApiResult, ClientError},
    repository::user_repository,
};
use axum::{
    headers::{authorization::Basic, Authorization},
    Extension, Json, Router, TypedHeader,
};
use axum_extra::routing::{RouterExt, TypedPath};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{instrument, Instrument};

pub fn user_routes() -> Router {
    Router::new().typed_post(login)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/login")]
pub struct LoginPath;

/// Creates a new item.
#[instrument(skip(db, basic_auth))]
pub async fn login(
    _: LoginPath,
    Extension(db): Extension<PgPool>,
    TypedHeader(basic_auth): TypedHeader<Authorization<Basic>>,
) -> ApiResult<Json<i32>> {
    tracing::info!("Fetching connection");
    let mut conn = db
        .acquire()
        .instrument(tracing::info_span!("acquire"))
        .await?;
    let username = basic_auth.username();
    let password = basic_auth.password();
    tracing::info!("Authenticating user");
    let id = user_repository::authenticate(&mut conn, username, password).await?;
    tracing::info!("Returning");
    let id = id.map(Json).ok_or(ClientError::Unauthorized)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use axum::{headers::Authorization, Extension, Json, TypedHeader};
    use sqlx::PgPool;

    use crate::{
        api::rest::user_api::{login, LoginPath},
        infra::error::{ApiError, ClientError},
    };

    #[sqlx::test]
    async fn user_with_correct_password_can_login(db: PgPool) {
        let username = "user";
        let password = "user";
        let basic_auth = TypedHeader(Authorization::basic(username, password));
        let Json(id) = login(LoginPath, Extension(db), basic_auth).await.unwrap();
        assert_eq!(1, id);
    }

    #[sqlx::test]
    async fn user_with_wrong_password_cannot_login(db: PgPool) {
        let username = "user";
        let password = "notuser";
        let basic_auth = TypedHeader(Authorization::basic(username, password));
        let error = login(LoginPath, Extension(db), basic_auth)
            .await
            .unwrap_err();
        assert_eq!(ApiError::ClientError(ClientError::Unauthorized), error);
    }
}
