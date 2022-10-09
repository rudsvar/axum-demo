//! An example web service with axum.

use axum_web_demo::{
    api::{grpc, rest},
    infra::config::{self, DatabaseConfig},
};
use sqlx::{
    pool::PoolOptions,
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions, PgPool,
};
use std::{net::TcpListener, time::Duration};
use tracing::log::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

fn init_logging() {
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,axum_web_demo=debug".into());

    let (non_blocking_stdout, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let stdout = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_stdout)
        .with_filter(EnvFilter::new(log_level));

    let file_appender = tracing_appender::rolling::hourly("./logs", "log.");
    let (non_blocking_file_appender, _guard) = tracing_appender::non_blocking(file_appender);
    let file_appender = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_file_appender)
        .json()
        .with_filter(EnvFilter::new("info,axum_web_demo=trace"));

    let reg = tracing_subscriber::registry()
        .with(stdout)
        .with(file_appender);

    reg.init();
}

fn init_db(config: &DatabaseConfig) -> PgPool {
    let mut db_options = PgConnectOptions::default()
        .username(&config.username)
        .password(&config.password)
        .host(&config.host)
        .port(config.port)
        .database(&config.database_name)
        .ssl_mode(PgSslMode::Prefer);
    db_options.log_statements(LevelFilter::Debug);
    let db: PgPool = PoolOptions::default()
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy_with(db_options);
    db
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    let config = config::load_config()?;
    let db = init_db(&config.database);

    // Start servers
    let listener = TcpListener::bind(&format!(
        "{}:{}",
        config.server.address, config.server.http_port
    ))?;
    let axum_server = tokio::spawn(rest::axum_server(listener, db));
    let grpc_addr = format!("{}:{}", config.server.grpc_address, config.server.grpc_port);
    let tonic_server = tokio::spawn(grpc::tonic_server(grpc_addr.parse()?));
    let _ = tokio::join!(axum_server, tonic_server);

    Ok(())
}
