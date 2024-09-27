//! An example web service with axum.

use axum_demo::infra::{self};
use sqlx::migrate::Migrator;
use std::time::Duration;
use tokio::net::TcpListener;

static MIGRATOR: Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    color_eyre::install()?;

    let config = infra::config::load_config()?;
    let _guard = infra::logging::init_logging(&config.logging);
    let db = infra::database::init_db(&config.database);

    // Run normal migrations
    while let Err(e) = MIGRATOR.run(&db).await {
        tracing::error!("Failed to run migrations: {}", e);
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
    tracing::info!("Completed normal migrations");

    // Start servers
    let http_address = &config.server.http_address;
    let http_port = &config.server.http_port;
    let addr = format!("{}:{}", http_address, http_port);
    let listener = TcpListener::bind(addr).await?;
    axum_demo::app::run_app(listener, db).await?;

    Ok(())
}
