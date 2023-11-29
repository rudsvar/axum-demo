//! Global application state.
//!
//! Used for access to common resources such as a
//! database pool or a preconfigured http client.

use super::{config::Config, database::DbPool};
use axum::extract::FromRef;
use reqwest::Client;

/// Global application state.
#[derive(Clone, Debug, FromRef)]
pub struct AppState {
    db: DbPool,
    client: Client,
    config: Config,
}

impl AppState {
    /// Constructs a new [`AppState`].
    pub fn new(db: DbPool, config: Config) -> Self {
        let client = reqwest::Client::new();
        Self { db, client, config }
    }

    /// Returns the database pool.
    pub fn db(&self) -> &DbPool {
        &self.db
    }

    /// Returns the HTTP client.
    pub fn http(&self) -> &Client {
        &self.client
    }

    /// Returns the application config.
    pub fn config(&self) -> &Config {
        &self.config
    }
}
