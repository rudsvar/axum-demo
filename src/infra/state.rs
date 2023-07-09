//! Global application state.
//!
//! Used for access to common resources such as a
//! database pool or a preconfigured http client.

use super::{config::Config, database::DbPool};
use crate::infra::http::HttpClient;
use axum::extract::FromRef;

/// Global application state.
#[derive(Clone, Debug, FromRef)]
pub struct AppState {
    db: DbPool,
    client: HttpClient,
    config: Config,
}

impl AppState {
    /// Constructs a new [`AppState`].
    pub fn new(db: DbPool, config: Config) -> Self {
        let client = reqwest::Client::new();
        let client = HttpClient::new(client, db.clone());
        Self { db, client, config }
    }

    /// Returns the database pool.
    pub fn db(&self) -> &DbPool {
        &self.db
    }

    /// Returns the HTTP client.
    pub fn http(&self) -> &HttpClient {
        &self.client
    }

    /// Returns the application config.
    pub fn config(&self) -> &Config {
        &self.config
    }
}
