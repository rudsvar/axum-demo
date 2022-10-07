//! An example web service with axum.

use axum::{
    body::{Body, Bytes},
    extract::Query,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tonic::Status;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

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

/// A name query parameter.
#[derive(Deserialize)]
pub struct Name {
    name: String,
}

/// This is a response to the hello endpoint.
#[derive(Serialize)]
pub struct HelloResponse {
    /// A personal greeting.
    greeting: String,
    /// Request counter.
    count: usize,
}

/// A handler for requests to the hello endpoint.
pub async fn hello_handler(
    Extension(i): Extension<Arc<AtomicUsize>>,
    Query(name): Query<Name>,
) -> Json<HelloResponse> {
    let prev = i.fetch_add(1, Ordering::SeqCst);
    Json(HelloResponse {
        greeting: name.name,
        count: prev,
    })
}

async fn axum_server() -> Result<(), hyper::Error> {
    let app = Router::new()
        .route("/", post(|| async move { "Hello from `POST /`" }))
        .route("/hello", get(hello_handler))
        .layer(middleware::from_fn(print_request_response))
        .layer(Extension(Arc::new(AtomicUsize::new(0))))
        .into_make_service();
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("Starting Axum on {}", addr);
    let axum_server = axum::Server::bind(&addr)
        .serve(app)
        .with_graceful_shutdown(async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!("Failed to fetch ctrl_c: {}", e);
            }
            tracing::info!("Axum shutting down");
        });
    axum_server.await
}
async fn tonic_server() -> Result<(), tonic::transport::Error> {
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

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axum_web_demo=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let axum_server = tokio::spawn(axum_server());
    let tonic_server = tokio::spawn(tonic_server());

    let res = tokio::try_join!(axum_server, tonic_server);
    match res {
        Ok((axum_result, tonic_result)) => {
            if let Err(e) = axum_result {
                tracing::error!("Axum server failed: {}", e);
            }
            if let Err(e) = tonic_result {
                tracing::error!("Tonic server failed: {}", e);
            }
        }
        Err(e) => tracing::error!("error joining tasks: {}", e),
    }
}

async fn print_request_response(
    req: hyper::Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody,
    B::Error: std::fmt::Display,
{
    let bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {} body: {}", direction, err),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}
