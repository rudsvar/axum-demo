//! Utilities for performing integration calls over HTTP.
//!
//! Examples include [`LogClient`] and [`logging_client`] for creating
//! HTTP clients that automatically log requests.

use reqwest::{Client, Request, Response};
use std::{future::Future, pin::Pin, time::Duration};
use tower::{Service, ServiceBuilder, ServiceExt};

use crate::{
    core::request::request_repository::{self, NewRequest},
    infra::{
        database::DbPool,
        error::{ApiError, ApiResult, InternalError},
    },
};

/// A HTTP client wrapper for pre- and post-processing requests.
#[derive(Clone, Debug)]
pub struct LogClient(Client, DbPool);

impl LogClient {
    /// Wraps a client.
    pub fn new(client: Client, db: DbPool) -> Self {
        Self(client, db)
    }
    /// Send a logged HTTP request.
    pub async fn send(&mut self, request: reqwest::Request) -> ApiResult<reqwest::Response> {
        self.ready().await?.call(request).await
    }
}

impl Service<Request> for LogClient {
    type Response = Response;
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let into_api_error = |e| ApiError::InternalError(InternalError::ReqwestError(e));
        self.0.poll_ready(cx).map_err(into_api_error)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut client = self.0.clone();
        let db = self.1.clone();
        Box::pin(async move {
            tracing::info!("Sending request: {} {}", req.method(), req.url());
            let method = req.method().to_string();
            let uri = req.url().path().to_string();
            let host = req
                .url()
                .host_str()
                .map(|h| h.to_string())
                .ok_or_else(|| InternalError::Other("missing host in client".to_string()))?;
            let mut request_body = None;
            if let Some(body) = req.body() {
                tracing::info!("Request body:\n{:?}", body);
                request_body = body
                    .as_bytes()
                    .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
            }
            // Perform call
            let res = client
                .call(req)
                .await
                .map_err(InternalError::ReqwestError)?;
            // Get response data
            let status = res.status();
            let headers = res.headers().clone();
            let bytes = res.bytes().await.map_err(InternalError::ReqwestError)?;
            // Log it
            let mut tx = db.begin().await?;
            let new_req = NewRequest {
                host,
                method,
                uri,
                request_body,
                response_body: String::from_utf8(bytes.to_vec()).ok(),
                status: status.as_u16() as i32,
            };
            let stored_req = request_repository::log_request(&mut tx, new_req).await?;
            tx.commit().await?;
            // Check if ok
            if status.is_success() {
                // Convert to http response
                let mut res = http::Response::builder()
                    .status(status)
                    .body(bytes)
                    .unwrap();
                *res.headers_mut() = headers;

                tracing::info!("Received response: {}", status);
                if !res.body().is_empty() {
                    tracing::info!("Body:\n{:?}", res.body());
                }

                Ok(Response::from(res))
            } else {
                tracing::error!("Received response: {}", status);
                Err(ApiError::InternalError(InternalError::IntegrationError(
                    format!("Request with id {} failed", stored_req.id),
                )))
            }
        })
    }
}

/// A preconfigured HTTP client.
pub fn logging_client(
    db: DbPool,
) -> impl Service<
    Request,
    Response = Response,
    Error = ApiError,
    Future = <LogClient as Service<Request>>::Future,
> {
    let client = reqwest::Client::new();
    ServiceBuilder::new()
        .rate_limit(1, Duration::from_secs(1))
        .layer_fn(|c| LogClient::new(c, db.clone()))
        .service(client)
}

#[cfg(test)]
mod tests {
    use super::logging_client;
    use http::StatusCode;
    use serde::Deserialize;
    use sqlx::PgPool;
    use tower::Service;

    #[derive(Debug, PartialEq, Deserialize)]
    struct Product {
        id: i32,
        title: String,
    }

    #[sqlx::test]
    #[ignore = "Does an integration call"]
    async fn log_client_logs(db: PgPool) {
        tracing_subscriber::fmt().init();
        let mut client = logging_client(db);

        let req = reqwest::Client::new()
            .get("https://dummyjson.com/products/1")
            .build()
            .unwrap();

        // Act
        let res = client.call(req).await.unwrap();

        // Assert
        assert_eq!(res.status(), StatusCode::OK);
        let product: Product = res.json().await.unwrap();
        assert_eq!(
            product,
            Product {
                id: 1,
                title: "iPhone 9".to_string()
            }
        );
    }
}
