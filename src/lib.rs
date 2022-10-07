pub mod grpc;
pub mod rest;
pub mod service;

/// Completes when when ctrl-c is pressed.
pub(crate) async fn shutdown(name: &str) {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to fetch ctrl_c: {}", e);
    }
    tracing::info!("{} shutting down", name);
}
