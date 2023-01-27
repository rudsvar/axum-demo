//! An example web service with axum.

use axum_demo::{
    infra::{self},
    integration, {grpc, rest},
};
use sqlx::migrate::Migrator;
use std::{net::TcpListener, time::Duration};

static MIGRATOR: Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv()?;

    let _guard = infra::logging::init_logging();
    let config = infra::config::load_config()?;
    let db = infra::database::init_db(&config.database);
    let mq = integration::mq::init_mq(&config.mq).await?;

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
    ))?;
    let axum_server = tokio::spawn(rest::axum_server(listener, db.clone(), mq, config.clone()));
    let grpc_addr = format!("{}:{}", config.server.grpc_address, config.server.grpc_port);
    let tonic_server = tokio::spawn(grpc::tonic_server(grpc_addr.parse()?, db.clone()));
    let _ = tokio::join!(axum_server, tonic_server);

    Ok(())
}
