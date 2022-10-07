use self::hello::{greeter_server::Greeter, HelloReply, HelloRequest};
use crate::service;
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
        let message = service::greeter::greet(&request.name);
        tracing::debug!("gRPC out: {}", message);

        let reply = hello::HelloReply { message };

        Ok(tonic::Response::new(reply)) // Send back our formatted greeting
    }
}

#[cfg(test)]
mod tests {
    use crate::api::grpc::greeter::{
        hello::{greeter_server::Greeter, HelloRequest},
        MyGreeter,
    };

    #[tokio::test]
    async fn greeter_test() {
        let greeter = MyGreeter {};
        let input = tonic::Request::new(HelloRequest {
            name: "World".to_string(),
        });
        let output = greeter.say_hello(input).await.unwrap();
        assert_eq!("Hello, World!", output.into_inner().message);
    }
}
