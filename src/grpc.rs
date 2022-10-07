use tonic::Status;
use crate::grpc::hello_world::greeter_server::GreeterServer;

use self::hello_world::{greeter_server::Greeter, HelloRequest, HelloReply};

pub mod hello_world {
    tonic::include_proto!("helloworld"); // The string specified here must match the proto package name
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: tonic::Request<HelloRequest>, // Accept request of type HelloRequest
    ) -> Result<tonic::Response<HelloReply>, Status> {
        let request = request.into_inner();

        // Return an instance of type HelloReply
        tracing::debug!("gRPC in: {}", request.name);
        let message = format!("Hello {}!", request.name);
        tracing::debug!("gRPC out: {}", message);

        let reply = hello_world::HelloReply { message };

        Ok(tonic::Response::new(reply)) // Send back our formatted greeting
    }
}

pub async fn tonic_server() -> Result<(), tonic::transport::Error> {
    let addr = "[::1]:50051".parse().unwrap();
    tracing::info!("Starting Tonic on {}", addr);
    let grpc_server = tonic::transport::Server::builder()
        .add_service(GreeterServer::new(MyGreeter::default()))
        .serve_with_shutdown(addr, async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!("Failed to fetch ctrl_c: {}", e);
            }
            tracing::info!("Tonic shutting down");
        });
    grpc_server.await
}
