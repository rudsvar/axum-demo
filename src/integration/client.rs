//! Utilities for performing integration calls over HTTP.
//!
//! Examples include [`LogClient`] and [`log_client`] for creating
//! HTTP clients that automatically log requests.

use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use reqwest::{Client, Request};
use serde::Deserialize;
use std::{future::Future, pin::Pin, string::FromUtf8Error};
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

/// A HTTP response.
#[derive(Debug)]
pub struct MyResponse {
    status: StatusCode,
    headers: HeaderMap,
    bytes: Bytes,
}

impl MyResponse {
    /// The HTTP status of the response.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// The headers of the response.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The bytes of the response.
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    /// The UTF-8 text of the response.
    /// Fails if the response is not valid UTF-8.
    pub fn text(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.bytes.to_vec())
    }

    /// Tries to parse the response as some deserializable type.
    pub fn json<'a, T: Deserialize<'a>>(&'a self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.bytes)
    }
}

impl Service<Request> for LogClient {
    type Response = MyResponse;
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
            // Send request
            tracing::info!("Sending request: {} {}", req.method(), req.url());
            let res = client
                .call(req)
                .await
                .map_err(InternalError::ReqwestError)?;
            let status = res.status();
            let headers = res.headers().clone();
            let bytes = res.bytes().await.map_err(InternalError::ReqwestError)?;
            // Check response
            if !status.is_success() {
                tracing::info!("Received response: {}", status);
                Ok(MyResponse {
                    status,
                    headers,
                    bytes,
                })
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
        // Get response
        let res = client.call(req).await.unwrap();
        let product: Product = res.json().unwrap();
        // Assertions
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            product,
            Product {
                id: 1,
                title: "iPhone 9".to_string()
            }
        )
    }
}
