//! An example web service with axum.

use axum_demo::{
    api::{grpc, rest},
    infra::{self},
};
use std::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = infra::logging::init_logging();
    let config = infra::config::load_config()?;
    let db = infra::database::init_db(&config.database);

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
