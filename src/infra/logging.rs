//! For setting up logging.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use super::config::JaegerConfig;

/// Flushes logs upon being dropped.
#[derive(Debug)]
pub struct LogGuard {
    _guards: Vec<WorkerGuard>,
}

/// Initializes logging.
pub fn init_logging(jaeger_config: &JaegerConfig) -> LogGuard {
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,tower_http=trace,axum_demo=debug".into());

    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let stdout = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_stdout)
        .with_filter(EnvFilter::new(log_level.clone()));

    let app_name = env!("CARGO_PKG_NAME");
    let jaeger_endpoint = format!("{}:{}", jaeger_config.host, jaeger_config.port);
    let opentelemetry_tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_endpoint(jaeger_endpoint)
        .with_service_name(app_name)
        .with_auto_split_batch(true)
        .with_max_packet_size(8192)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();
    let opentelemetry = tracing_opentelemetry::layer()
        .with_tracer(opentelemetry_tracer)
        .with_filter(EnvFilter::new(log_level));

    let console_layer = if cfg!(debug_assertions) {
        Some(console_subscriber::spawn())
    } else {
        None
    };

    let reg = tracing_subscriber::registry()
        .with(stdout)
        .with(opentelemetry)
        .with(console_layer)
        .with(ErrorLayer::default());

    reg.init();

    LogGuard {
        _guards: vec![stdout_guard],
    }
}
