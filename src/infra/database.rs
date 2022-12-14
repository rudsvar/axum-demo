//! For interacting with the database.

use super::{config::DatabaseConfig, error::ApiError};
use sqlx::{
    pool::PoolOptions,
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions, PgPool, Postgres,
};
use std::time::Duration;
use tracing::log::LevelFilter;

/// A transaction type that implements [`axum::extract::FromRequest`].
/// Will automatically commit on save, and abort on failure.
pub type NewTx = axum_sqlx_tx::Tx<Postgres, ApiError>;

/// A common transaction type.
/// Use this for the business and persistence layer.
pub type Tx = sqlx::Transaction<'static, Postgres>;

/// A common database pool type.
pub type DbPool = PgPool;

/// Connects to the database based on some configuration.
pub fn init_db(config: &DatabaseConfig) -> PgPool {
    let mut db_options = PgConnectOptions::default()
        .username(&config.username)
        .password(&config.password)
        .host(&config.host)
        .port(config.port)
        .database(&config.database_name)
        .ssl_mode(PgSslMode::Prefer);
    db_options.log_statements(LevelFilter::Debug);
    let db: PgPool = PoolOptions::default()
        .acquire_timeout(Duration::from_secs(1))
        .min_connections(1)
        .max_connections(100)
        .connect_lazy_with(db_options);
    db
}
