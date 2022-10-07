//! An example web service with axum.

use axum_web_demo::{grpc, rest};
use sqlx::{
    pool::PoolOptions,
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions, PgPool,
};
use std::{net::TcpListener, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set up logging
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,axum_web_demo=debug".into());

    let stdout = tracing_subscriber::fmt::layer().with_filter(EnvFilter::new(log_level));

    let file_appender = tracing_appender::rolling::hourly("./logs", "log.");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let file_writer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .json()
        .with_filter(EnvFilter::new("info,axum_web_demo=trace"));

    let reg = tracing_subscriber::registry()
        .with(stdout)
        .with(file_writer);

    reg.init();

    // Configure database connection
    let mut db_options = PgConnectOptions::default()
        .username("postgres")
        .password("password")
        .host("localhost")
        .port(5432)
        .database("axum-web-demo")
        .ssl_mode(PgSslMode::Prefer);
    db_options.log_statements(tracing::log::LevelFilter::Debug);
    let db: PgPool = PoolOptions::default()
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy_with(db_options);

    // Start servers
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    let axum_server = tokio::spawn(rest::axum_server(listener, db));
    let tonic_server = tokio::spawn(grpc::tonic_server("[::1]:50051".parse()?));
    let _ = tokio::join!(axum_server, tonic_server);

    Ok(())
}
