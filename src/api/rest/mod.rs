use crate::shutdown;
use axum::{
    body::Bytes,
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
    Router, Extension,
};
use hyper::{Body, Request, Response, StatusCode};
use sqlx::PgPool;
use std::net::TcpListener;

pub mod hello;
pub mod item_api;

pub async fn axum_server(addr: TcpListener, db: PgPool) -> Result<(), hyper::Error> {
    let app = Router::new()
        .route("/", post(|| async move { "Hello from `POST /`" }))
        .nest("/", hello::hello_routes())
        .nest("/", item_api::item_routes())
        .layer(Extension(db))
        .layer(middleware::from_fn(print_request_response))
        .into_make_service();
    tracing::info!("Starting Axum on {:?}", addr.local_addr());
    let axum_server = axum::Server::from_tcp(addr)?
        .serve(app)
        .with_graceful_shutdown(shutdown("axum"));
    axum_server.await
}

async fn print_request_response(
    req: hyper::Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("Request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("Response", body).await?;
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
        tracing::trace!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}
