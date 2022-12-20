//! Global application state.
//!
//! Used for access to common resources such as a
//! database pool or a preconfigured http client.

use super::database::DbPool;
use crate::integration::client::LogClient;
use axum::extract::FromRef;

/// Global application state.
#[derive(Clone, Debug, FromRef)]
pub struct AppState {
    db: DbPool,
    client: LogClient,
}

impl AppState {
    /// Constructs a new [`AppState`].
    pub fn new(db: DbPool) -> Self {
        let client = reqwest::Client::new();
        let client = LogClient::new(client, db.clone());
        Self { db, client }
    }

    /// Returns the database pool.
    pub fn db(&self) -> &DbPool {
        &self.db
    }

    /// Returns the HTTP client.
    pub fn client(&self) -> &LogClient {
        &self.client
    }
}
