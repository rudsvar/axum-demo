//! For setting up logging.

use opentelemetry::KeyValue;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    trace::{BatchSpanProcessor, SdkTracerProvider},
    Resource,
};
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

    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(jaeger_endpoint)
        .build()
        .unwrap();

    let processor = BatchSpanProcessor::builder(exporter).build();

    let provider = SdkTracerProvider::builder()
        .with_span_processor(processor)
        .with_resource(
            Resource::builder_empty()
                .with_attributes([KeyValue::new("service.name", app_name)])
                .build(),
        )
        .build();

    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, app_name);
    opentelemetry::global::set_tracer_provider(provider);

    let opentelemetry = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(EnvFilter::new(log_level));

    let reg = tracing_subscriber::registry()
        .with(stdout)
        .with(opentelemetry)
        .with(ErrorLayer::default());

    reg.init();

    LogGuard {
        _guards: vec![stdout_guard],
    }
}
