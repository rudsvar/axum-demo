//! Global application state.
//!
//! Used for access to common resources such as a
//! database pool or a preconfigured http client.

use super::database::DbPool;
use crate::integration::{http::HttpClient, mq::MqPool};
use axum::extract::FromRef;

/// Global application state.
#[derive(Clone, Debug, FromRef)]
pub struct AppState {
    db: DbPool,
    client: HttpClient,
    mq: MqPool,
}

impl AppState {
    /// Constructs a new [`AppState`].
    pub fn new(db: DbPool, mq: MqPool) -> Self {
        let client = reqwest::Client::new();
        let client = HttpClient::new(client, db.clone());
        Self { db, client, mq }
    }

    /// Returns the database pool.
    pub fn db(&self) -> &DbPool {
        &self.db
    }

    /// Returns the HTTP client.
    pub fn http(&self) -> &HttpClient {
        &self.client
    }

    /// Returns the MQ connection.
    pub fn mq(&self) -> &MqPool {
        &self.mq
    }
}
