use super::{
    database::DbPool,
    error::{ApiError, ApiResult, ClientError, InternalError},
};
use axum::{
    async_trait,
    extract::FromRequest,
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use sqlx::PgConnection;
use std::marker::PhantomData;
use tracing::instrument;

#[derive(Debug)]
pub struct Any;

#[derive(Debug)]
pub struct Admin;

#[derive(Debug)]
pub struct User<Role = Any> {
    id: i32,
    role: String,
    role_type: PhantomData<Role>,
}

impl<Role> User<Role> {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn role(&self) -> &str {
        self.role.as_ref()
    }
}

#[async_trait]
impl<B> FromRequest<B> for User
where
    B: Send,
{
    type Rejection = ApiError;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        // Get authorization header
        let TypedHeader(auth) = req
            .extract::<TypedHeader<Authorization<Basic>>>()
            .await
            .map_err(|_| ClientError::Unauthorized)?;

        // Get db connection
        let db = req
            .extensions()
            .get::<DbPool>()
            .ok_or_else(|| InternalError::MissingExtension("database pool".to_string()))?
            .clone();
        let mut tx = db.acquire().await.map_err(InternalError::SqlxError)?;

        // Authenticate user
        let user = authenticate(&mut tx, auth.username(), auth.password()).await?;

        Ok(user)
    }
}

#[async_trait]
impl<B> FromRequest<B> for User<Admin>
where
    B: Send,
{
    type Rejection = ApiError;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let user = req.extract::<User>().await?;
        if user.role() == "admin" {
            Ok(User {
                id: user.id,
                role: user.role,
                role_type: PhantomData::default(),
            })
        } else {
            Err(ClientError::Forbidden)?
        }
    }
}

struct UserRow {
    pub id: i32,
    pub password: String,
    pub role: String,
}

/// Validate a user's password.
#[instrument(skip(conn, password))]
pub async fn authenticate(
    conn: &mut PgConnection,
    username: &str,
    password: &str,
) -> ApiResult<User> {
    tracing::info!("Fetching {}'s password", username);
    let user = sqlx::query_as!(
        UserRow,
        r#"
        SELECT id, password, role FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_one(conn)
    .await?;

    tracing::info!("Verifying password");
    let password_is_ok = bcrypt::verify(password, &user.password)?;
    if password_is_ok {
        Ok(User {
            id: user.id,
            role: user.role,
            role_type: PhantomData::default(),
        })
    } else {
        Err(ClientError::Unauthorized.into())
    }
}

#[cfg(test)]
mod tests {
    use super::authenticate;
    use crate::infra::error::{ApiError, ClientError};
    use sqlx::{pool::PoolConnection, Postgres};

    #[sqlx::test(fixtures("users"))]
    async fn user_with_correct_password_can_login(mut conn: PoolConnection<Postgres>) {
        let username = "user";
        let password = "user";
        let user = authenticate(&mut conn, username, password).await.unwrap();
        assert_eq!(1, user.id());

        let username = "admin";
        let password = "admin";
        let user = authenticate(&mut conn, username, password).await.unwrap();
        assert_eq!(2, user.id());
    }

    #[sqlx::test(fixtures("users"))]
    async fn user_with_incorrect_password_can_login(mut conn: PoolConnection<Postgres>) {
        let username = "user";
        let password = "notuser";
        let result = authenticate(&mut conn, username, password).await;
        assert!(matches!(
            result,
            Err(ApiError::ClientError(ClientError::Unauthorized))
        ))
    }
}
