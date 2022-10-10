use sqlx::PgConnection;
use tracing::instrument;

struct User {
    pub id: i32,
    pub password: String,
}

/// Validate a user's password.
#[instrument(skip(tx, password))]
pub async fn authenticate(tx: &mut PgConnection, username: &str, password: &str) -> Option<i32> {
    tracing::info!("Fetching {}'s password", username);
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, password FROM users
        WHERE username = $1
        "#,
        username
    )
    .fetch_one(tx)
    .await
    .unwrap();

    tracing::info!("Verifygin password");
    if let Ok(true) = bcrypt::verify(password, &user.password) {
        Some(user.id)
    } else {
        None
    }
}
