//! gRPC API implementations.

use crate::{
    grpc::greeter::{hello::greeter_server::GreeterServer, MyGreeter},
    shutdown,
};
use std::net::SocketAddr;

pub mod greeter;

/// Starts a tonic server serving our gRPC API on the specified address.
pub async fn tonic_server(addr: SocketAddr) -> Result<(), tonic::transport::Error> {
    tracing::info!("Starting tonic on {}", addr);
    let grpc_server = tonic::transport::Server::builder()
        .add_service(GreeterServer::new(MyGreeter::default()))
        .serve_with_shutdown(addr, shutdown("tonic"));
    grpc_server.await
}
