use self::hello::{greeter_server::Greeter, HelloReply, HelloRequest};
use tonic::Status;

pub mod hello {
    tonic::include_proto!("hello"); // The string specified here must match the proto package name
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

        let reply = hello::HelloReply { message };

        Ok(tonic::Response::new(reply)) // Send back our formatted greeting
    }
}
