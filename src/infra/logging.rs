//! For setting up logging.

use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use super::config::LoggingConfig;

/// Flushes logs upon being dropped.
#[derive(Debug)]
pub struct LogGuard {
    _guards: Vec<WorkerGuard>,
}

/// Initializes logging.
pub fn init_logging(config: &LoggingConfig) -> LogGuard {
    let log_level = &config.rust_log;

    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let stdout = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_stdout)
        .with_filter(EnvFilter::new(log_level.clone()));

    let app_name = env!("CARGO_PKG_NAME");
    let jaeger_endpoint = format!("{}:{}", config.jaeger_host, config.jaeger_port);
    let opentelemetry_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(jaeger_endpoint),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::config()
                .with_resource(Resource::new(vec![KeyValue::new("service.name", app_name)])),
        )
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
