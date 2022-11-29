//! Utilities for performing integration calls over HTTP.
//!
//! Examples include [`LogClient`] and [`log_client`] for creating
//! HTTP clients that automatically log requests.

use reqwest::{Client, Request};
use std::{future::Future, pin::Pin};
use tower::{Service, ServiceBuilder};

use crate::infra::error::{ApiError, ClientError, InternalError};

/// A HTTP client wrapper for pre- and post-processing requests.
#[derive(Debug)]
pub struct LogClient(Client);

impl LogClient {
    /// Wraps a client.
    pub fn new(client: Client) -> Self {
        Self(client)
    }
}

impl Service<reqwest::Request> for LogClient {
    type Response = reqwest::Response;
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let into_api_error = |e| ApiError::InternalError(InternalError::ReqwestError(e));
        self.0.poll_ready(cx).map_err(into_api_error)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut client = self.0.clone();
        Box::pin(async move {
            tracing::info!("Sending request: {} {}", req.method(), req.url());
            tracing::info!("Body: {:?}", req.body());
            // Perform call
            let res = client
                .call(req)
                .await
                .map_err(InternalError::ReqwestError)?;
            // Get response data
            let status = res.status();
            let headers = res.headers().clone();
            let bytes = res.bytes().await.map_err(InternalError::ReqwestError)?;
            // Check if ok
            if status.is_success() {
                // Convert to http response
                let mut res = http::Response::builder()
                    .status(status)
                    .body(bytes)
                    .unwrap();
                *res.headers_mut() = headers;

                tracing::info!("Received response: {}", status);
                tracing::info!("Body: {:?}", res.body());

                Ok(reqwest::Response::from(res))
            } else {
                tracing::error!("Received response: {}", status);
                Err(ApiError::ClientError(ClientError::IntegrationError))
            }
        })
    }
}

/// A preconfigured HTTP client.
pub fn log_client() -> LogClient {
    let client = reqwest::Client::new();
    ServiceBuilder::new()
        .layer_fn(LogClient::new)
        .service(client)
}

#[cfg(test)]
mod tests {
    use super::log_client;
    use http::{Method, StatusCode};
    use reqwest::{Request, Url};
    use serde::Deserialize;
    use tower::Service;

    #[derive(Debug, PartialEq, Deserialize)]
    struct Product {
        id: i32,
        title: String,
    }

    #[tokio::test]
    #[ignore = "Does an integration call"]
    async fn log_client_logs() {
        tracing_subscriber::fmt().init();
        let mut client = log_client();
        // Create request
        let req = Request::new(
            Method::GET,
            Url::parse("https://dummyjson.com/products/1").unwrap(),
        );

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
        )
    }
}
