//! For interacting with the database.

use super::config::DatabaseConfig;
use sqlx::{
    pool::{PoolConnection, PoolOptions},
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions, PgPool, Postgres,
};
use std::time::Duration;
use tracing::log::LevelFilter;

/// A common transaction type.
/// Use this for the business and persistence layer.
pub type Tx = sqlx::Transaction<'static, Postgres>;

/// A common database pool type.
pub type DbPool = PgPool;

/// A common database connection type.
pub type DbConnection = PoolConnection<Postgres>;

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
    db_options.log_slow_statements(LevelFilter::Warn, Duration::from_secs(1));
    let db: PgPool = PoolOptions::default()
        .acquire_timeout(Duration::from_secs(1))
        .min_connections(1)
        .max_connections(10)
        .idle_timeout(Duration::from_secs(10 * 60))
        .max_lifetime(Duration::from_secs(30 * 60))
        .connect_lazy_with(db_options);
    db
}
