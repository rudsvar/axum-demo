//! For setting up logging.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Flushes logs upon being dropped.
#[derive(Debug)]
pub struct LogGuard {
    _guards: Vec<WorkerGuard>,
}

/// Initializes logging.
pub fn init_logging() -> LogGuard {
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,axum_demo=debug".into());
    let log_level_file = "info,axum_demo=trace";

    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let stdout = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_stdout)
        .with_filter(EnvFilter::new(&log_level));

    let file_appender = tracing_appender::rolling::hourly("./logs", "log.");
    let (non_blocking_file_appender, file_guard) = tracing_appender::non_blocking(file_appender);
    let file_appender = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_file_appender)
        .json()
        .with_filter(EnvFilter::new(&log_level_file));

    let app_name = env!("CARGO_PKG_NAME");
    let opentelemetry_tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(app_name)
        .install_simple()
        .unwrap();
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(opentelemetry_tracer);

    let reg = tracing_subscriber::registry()
        .with(stdout)
        .with(file_appender)
        .with(opentelemetry);

    reg.init();

    LogGuard {
        _guards: vec![stdout_guard, file_guard],
    }
}
