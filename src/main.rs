//! An example web service with axum.

use axum_demo::infra::{self};
use sqlx::migrate::Migrator;
use std::time::Duration;
use tokio::net::TcpListener;

static MIGRATOR: Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    color_eyre::install()?;

    let _guard = infra::logging::init_logging();
    let config = infra::config::load_config()?;
    let db = infra::database::init_db(&config.database);

    // Run migrations
    tracing::info!("Running migrations");
    while let Err(e) = MIGRATOR.run(&db).await {
        tracing::error!("Failed to run migrations: {}", e);
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
    tracing::info!("Completed migrations");

    // Start servers
    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.server.http_address, config.server.http_port
    ))
    .await?;
    axum_demo::server::run_app(listener, db.clone(), config.clone()).await?;

    Ok(())
}
