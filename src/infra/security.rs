//! Types and utility functions relating to security.

use super::{
    database::Tx,
    error::{ApiError, ApiResult, ClientError, InternalError},
};
use axum::{
    async_trait,
    extract::FromRequest,
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use sqlx::Postgres;
use std::marker::PhantomData;
use tracing::instrument;

const ADMIN_ROLE: &str = "admin";

/// Any user role.
#[derive(Clone, Copy, Debug)]
pub struct Any;

/// The admin role.
#[derive(Clone, Copy, Debug)]
pub struct Admin;

/// An authenticated user.
/// This can only be constructed from a request.
pub struct User<Role = Any> {
    id: i32,
    role: String,
    role_type: PhantomData<Role>,
}

impl<Role> User<Role> {
    /// The id of the user.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The role of the user.
    pub fn role(&self) -> &str {
        self.role.as_ref()
    }

    /// Try to upgrade the user to the administrator type.
    /// This will only work if the user has the admin role.
    pub fn try_into_admin(self) -> Result<User<Admin>, ClientError> {
        if self.role() == ADMIN_ROLE {
            Ok(User {
                id: self.id,
                role: self.role,
                role_type: PhantomData,
            })
        } else {
            Err(ClientError::Forbidden)
        }
    }
}

impl User<Admin> {
    /// "Downgrade" an administrator.
    pub fn into_any(self) -> User {
        User {
            id: self.id,
            role: self.role,
            role_type: PhantomData,
        }
    }
}

impl<Role> std::fmt::Debug for User<Role> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("role", &self.role)
            .finish()
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
        let mut tx = req
            .extract::<axum_sqlx_tx::Tx<Postgres>>()
            .await
            .map_err(|_| InternalError::MissingExtension("transaction".to_string()))?;

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
        let admin = user.try_into_admin()?;
        Ok(admin)
    }
}

struct UserRow {
    pub id: i32,
    pub password: String,
    pub role: String,
}

/// Validate a user's password.
#[instrument(skip(conn, password))]
pub async fn authenticate(conn: &mut Tx, username: &str, password: &str) -> ApiResult<User> {
    tracing::info!("Fetching {}'s password", username);
    let user = sqlx::query_as!(
        UserRow,
        r#"
        SELECT id, password, role FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_optional(conn)
    .await?
    .ok_or(ClientError::Unauthorized)?;

    tracing::info!("Verifying password");
    let password_is_ok = bcrypt::verify(password, &user.password)?;
    if password_is_ok {
        Ok(User {
            id: user.id,
            role: user.role,
            role_type: PhantomData,
        })
    } else {
        Err(ClientError::Unauthorized.into())
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::authenticate;
    use crate::infra::{
        database::DbPool,
        error::{ApiError, ClientError},
        security::{Admin, User},
    };

    #[sqlx::test]
    async fn user_with_correct_password_can_login(db: DbPool) {
        let mut tx = db.begin().await.unwrap();
        let username = "user";
        let password = "user";
        let user = authenticate(&mut tx, username, password).await.unwrap();
        assert_eq!(1, user.id());

        let username = "admin";
        let password = "admin";
        let user = authenticate(&mut tx, username, password).await.unwrap();
        assert_eq!(2, user.id());
    }

    #[sqlx::test]
    async fn user_with_incorrect_password_can_login(db: DbPool) {
        let mut tx = db.begin().await.unwrap();
        let username = "user";
        let password = "notuser";
        let result = authenticate(&mut tx, username, password).await;
        assert!(matches!(
            result,
            Err(ApiError::ClientError(ClientError::Unauthorized))
        ))
    }

    fn user() -> User {
        User {
            id: 0,
            role: "admin".into(),
            role_type: PhantomData,
        }
    }

    fn admin() -> User<Admin> {
        user().try_into_admin().unwrap()
    }

    #[test]
    fn user_can_call_user_fn() {
        fn f(_: User) {}
        f(user());
    }

    #[test]
    fn admin_can_call_user_fn() {
        fn f<R>(_: User<R>) {}
        f(admin());
    }

    #[test]
    fn admin_can_call_user_fn_2() {
        fn user(_: User) {}
        user(admin().into_any());
    }
}
