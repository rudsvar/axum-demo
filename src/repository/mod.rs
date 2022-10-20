use sqlx::{PgConnection, PgPool, Postgres, Transaction};

pub mod item_repository;
pub mod user_repository;

pub type Tx = Transaction<'static, Postgres>;
pub type DbConnection = PgConnection;
pub type DbPool = PgPool;
