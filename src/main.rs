//! An example web service with axum.

use axum_web_demo::{grpc, rest};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,axum_web_demo=debug".into());

    let stdout = tracing_subscriber::fmt::layer().with_filter(EnvFilter::new(log_level));

    let file_appender = tracing_appender::rolling::minutely("./logs", "prefix.log");
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

    let axum_server = tokio::spawn(rest::axum_server());
    let tonic_server = tokio::spawn(grpc::tonic_server());
    let _ = tokio::join!(axum_server, tonic_server);

    Ok(())
}
