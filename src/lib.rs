#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    rust_2018_idioms,
    missing_docs
)]

//! A web service example with axum.
//!
//! To start it, you'll first need a database, then you have to run
//! any missing migrations, and finally run the application itself.
//! All three steps are listed below.
//!
//! ```shell
//! docker compose up -d db
//! sqlx database setup
//! cargo run
//! ```
//!
//! You can install `sqlx` with `cargo install sqlx-cli`.
//! When the application is up and running, visit `localhost:8080`.

pub mod core;
pub mod graphql;
pub mod grpc;
pub mod infra;
pub mod integration;
pub mod rest;

/// Completes when when ctrl-c is pressed.
pub(crate) async fn shutdown(name: &str) {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to fetch ctrl_c: {}", e);
    }
    tracing::info!("{} shutting down", name);
}
