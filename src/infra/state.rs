//! Global application state.
//!
//! Used for access to common resources such as a
//! database pool or a preconfigured http client.

use std::sync::Arc;

use super::database::DbPool;
use crate::integration::http::HttpClient;
use axum::extract::FromRef;
use lapin::Connection;

/// Global application state.
#[derive(Clone, Debug, FromRef)]
pub struct AppState {
    db: DbPool,
    client: HttpClient,
    mq: Arc<Connection>,
}

impl AppState {
    /// Constructs a new [`AppState`].
    pub fn new(db: DbPool, mq: Connection) -> Self {
        let client = reqwest::Client::new();
        let client = HttpClient::new(client, db.clone());
        Self {
            db,
            client,
            mq: Arc::new(mq),
        }
    }

    /// Returns the database pool.
    pub fn db(&self) -> &DbPool {
        &self.db
    }

    /// Returns the HTTP client.
    pub fn client(&self) -> &HttpClient {
        &self.client
    }

    /// Returns the MQ connection.
    pub fn mq(&self) -> &Connection {
        &self.mq
    }
}
