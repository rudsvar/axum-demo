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
}

impl AppState {
    /// Constructs a new [`AppState`].
    pub fn new(db: DbPool) -> Self {
        let client = reqwest::Client::new();
        Self { db, client }
    }

    /// Returns the database pool.
    pub fn db(&self) -> &DbPool {
        &self.db
    }

    /// Returns the HTTP client.
    pub fn http(&self) -> &Client {
        &self.client
    }

    /// Loads the application configuration.
    pub fn config(&self) -> color_eyre::Result<Config> {
        crate::infra::config::load_config()
    }
}
