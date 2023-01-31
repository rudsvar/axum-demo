//! An example web service with axum.

use axum_demo::{
    infra::{self},
    integration, {grpc, rest},
};
use sqlx::migrate::Migrator;
use std::{net::TcpListener, time::Duration};
use tokio_cron_scheduler::{Job, JobScheduler};

static MIGRATOR: Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    color_eyre::install()?;

    // Start scheduled task
    let sched = JobScheduler::new().await?;
    let job = Job::new_async("* * */1 * * *", |_, _| {
        Box::pin(async move {
            tracing::info!("Doing asynchronous check...");
            tokio::time::sleep(Duration::from_secs(5)).await;
            tracing::info!("Done");
        })
    })?;
    sched.add(job).await?;
    sched.start().await?;

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
