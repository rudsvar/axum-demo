//! Types and utility functions relating to security.
//!
//! If your API requires information about the calling user,
//! you can add a [`User<Role>`] to your parameter list.
//! An instance of this class can only be created from a request with appropriate
//! login information through the [`axum::extract::FromRequest`] trait.
//!
//! You can also add a type parameter representing the role of the user
//! to limit access to your method. The authentication and authorization
//! requirements are thus verified at compile time.
//!
//! # Examples
//!
//! ```
//! # use axum::Json;
//! # use axum_demo::infra::security::{User, Role};
//! # use axum_demo::infra::error::ApiResult;
//! /// A custom role.
//! struct CustomRole;
//!
//! impl Role for CustomRole {
//!     fn is_satisfied(role: &[&str]) -> bool {
//!         role.contains(&"foo") && role.contains(&"bar") || role.contains(&"baz")
//!     }
//! }
//!
//! /// A handler that guarantees that
//! /// 1. the user has been authenticated, and that
//! /// 2. the user has the [`CustomRole`] role.
//! pub async fn custom(user: User<CustomRole>) -> ApiResult<Json<i32>> {
//!     tracing::info!("Custom user logged in");
//!     Ok(Json(user.id()))
//! }
//! ```

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
use cached::proc_macro::cached;
use sqlx::Postgres;
use std::marker::PhantomData;
use tracing::instrument;

const ADMIN_ROLE: &str = "admin";

/// A trait to implement to create new roles.
///
/// # Examples
///
/// ```
/// # use axum::Json;
/// # use axum_demo::infra::security::{User, Role};
/// /// A custom role.
/// struct CustomRole;
///
/// impl Role for CustomRole {
///     fn is_satisfied(role: &[&str]) -> bool {
///         role.contains(&"foo") && role.contains(&"bar") || role.contains(&"baz")
///     }
/// }
/// ```
pub trait Role
where
    Self: Sized,
{
    /// Checks if the role is satisfied.
    fn is_satisfied(roles: &[&str]) -> bool;
}

/// Represents a user with unknown roles.
///
/// Using this in the parameter list means that we impose no restriction on the user's roles.
#[derive(Clone, Copy, Debug)]
pub struct Unknown;

impl Role for Unknown {
    fn is_satisfied(_: &[&str]) -> bool {
        true
    }
}

/// Represents a user with the administrator role.
///
/// Using this in the parameter list means that only administrators can call the function.
#[derive(Clone, Copy, Debug)]
pub struct Admin;

impl Role for Admin {
    fn is_satisfied(role: &[&str]) -> bool {
        role.contains(&ADMIN_ROLE)
    }
}

/// An authenticated user with role `R`, which defaults to [`Unknown`].
///
/// Having an instance of [`User<R>`] guarantees that the
/// calling user's information is stored in its fields,
/// and that the user has roles corresponding to `R`.
/// See [`Unknown`] and [`Admin`] for examples.
///
/// # Examples
///
/// ```
/// # use axum::Json;
/// # use axum_demo::infra::security::{Role, User};
/// # use axum_demo::infra::error::ApiResult;
/// # /// A custom role.
/// # struct CustomRole;
/// # impl Role for CustomRole {
/// #     fn is_satisfied(role: &[&str]) -> bool {
/// #         role.contains(&"foo") && role.contains(&"bar") || role.contains(&"baz")
/// #     }
/// # }
/// /// A handler that guarantees that
/// /// 1. the user has been authenticated, and that
/// /// 2. the user has the [`CustomRole`] role.
/// pub async fn custom(user: User<CustomRole>) -> ApiResult<Json<i32>> {
///     tracing::info!("Custom user logged in");
///     Ok(Json(user.id()))
/// }
/// ```
///
/// You can also attempt to upgrade a user at runtime,
/// which will verify that the user has the appropriate roles
/// required for the new user type.
///
/// ```
/// # use axum_demo::infra::security::{User, Admin};
/// # use axum_demo::infra::error::{ApiError, ClientError};
/// # use core::marker::PhantomData;
/// pub fn try_upgrade(user: User) {
///     let admin: Result<User<Admin>, ApiError> = user.try_upgrade();
/// }
/// ```
#[derive(Clone)]
pub struct User<R = Unknown> {
    id: i32,
    role: String,
    role_type: PhantomData<R>,
}

impl<R> User<R> {
    /// The id of the user.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The role of the user.
    pub fn role(&self) -> &str {
        self.role.as_ref()
    }

    /// Attempt to upgrade (or downgrade) a user's roles.
    pub fn try_upgrade<NewRole>(self) -> ApiResult<User<NewRole>>
    where
        NewRole: Role,
    {
        if NewRole::is_satisfied(&[&self.role]) {
            Ok(User {
                id: self.id,
                role: self.role,
                role_type: PhantomData,
            })
        } else {
            Err(ApiError::ClientError(ClientError::Forbidden))
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

impl<R> std::fmt::Debug for User<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("role", &self.role)
            .finish()
    }
}

#[async_trait]
impl<B, R> FromRequest<B> for User<R>
where
    B: Send,
    R: Role,
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

        // Make sure they have the correct roles
        let user = user.try_upgrade()?;

        Ok(user)
    }
}

struct UserRow {
    pub id: i32,
    pub password: String,
    pub role: String,
}

/// Validate a user's password.
#[cached(
    size = 100,
    time = 30,
    time_refresh,
    sync_writes = true,
    key = "String",
    convert = r##"{ format!("{}:{}", username, password) }"##,
    result = true
)]
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
        user().try_upgrade().unwrap()
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
