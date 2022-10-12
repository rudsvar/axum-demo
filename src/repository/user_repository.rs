use sqlx::PgConnection;
use tracing::instrument;

use crate::infra::error::ApiResult;

struct User {
    pub id: i32,
    pub password: String,
}

/// Validate a user's password.
#[instrument(skip(conn, password))]
pub async fn authenticate(
    conn: &mut PgConnection,
    username: &str,
    password: &str,
) -> ApiResult<Option<i32>> {
    tracing::info!("Fetching {}'s password", username);
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, password FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_one(conn)
    .await?;

    tracing::info!("Verifying password");
    let user_id = if let Ok(true) = bcrypt::verify(password, &user.password) {
        Some(user.id)
    } else {
        None
    };

    Ok(user_id)
}
