//! For interacting with the database.

use super::config::DatabaseConfig;
use sqlx::{
    pool::PoolOptions,
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions, PgConnection, PgPool, Postgres, Transaction,
};
use std::time::Duration;
use tracing::log::LevelFilter;

/// A common transaction type.
/// Use this for the business and persistence layer.
pub type Tx = Transaction<'static, Postgres>;

/// A common database connection type.
pub type DbConnection = PgConnection;

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
        .acquire_timeout(Duration::from_secs(5))
        .min_connections(1)
        .max_connections(100)
        .connect_lazy_with(db_options);
    db
}
