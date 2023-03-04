//! gRPC API implementation with tonic.

use crate::{
    infra::database::DbPool,
    shutdown,
    grpc::greeter::{hello::greeter_server::GreeterServer, MyGreeter},
    grpc::item::{item::item_service_server::ItemServiceServer, ItemServiceImpl},
};
use std::net::SocketAddr;

pub mod greeter;
pub mod item;

/// Starts a tonic server serving our gRPC API on the specified address.
pub async fn tonic_server(addr: SocketAddr, db: DbPool) -> Result<(), tonic::transport::Error> {
    tracing::info!("Starting tonic on {}", addr);
    let grpc_server = tonic::transport::Server::builder()
        .add_service(GreeterServer::new(MyGreeter::default()))
        .add_service(ItemServiceServer::new(ItemServiceImpl::new(db)))
        .serve_with_shutdown(addr, shutdown("tonic"));
    grpc_server.await
}
