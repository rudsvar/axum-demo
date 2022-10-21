use sqlx::{PgConnection, PgPool, Postgres, Transaction};

pub type Tx = Transaction<'static, Postgres>;
pub type DbConnection = PgConnection;
pub type DbPool = PgPool;
