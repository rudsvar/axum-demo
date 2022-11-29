//! Utilities for performing integration calls over HTTP.
//!
//! Examples include [`LogClient`] and [`log_client`] for creating
//! HTTP clients that automatically log requests.

use reqwest::{Client, Request, Response};
use std::{future::Future, pin::Pin, time::Duration};
use tower::{Service, ServiceBuilder};

use crate::{
    infra::{
        database::DbPool,
        error::{ApiError, ClientError, InternalError},
    },
    repository::request_repository::NewRequest,
};

/// A HTTP client wrapper for pre- and post-processing requests.
#[derive(Debug)]
pub struct LogClient(Client, DbPool);

impl LogClient {
    /// Wraps a client.
    pub fn new(client: Client, db: DbPool) -> Self {
        Self(client, db)
    }
}

impl Service<Request> for LogClient {
    type Response = Response;
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
        let db = self.1.clone();
        Box::pin(async move {
            tracing::info!("Sending request: {} {}", req.method(), req.url());
            let uri = req.url().path().to_string() + "?" + req.url().path();
            let mut request_body = None;
            if let Some(body) = req.body() {
                tracing::info!("Request body:\n{:?}", body);
                request_body = Some(String::from_utf8(body.as_bytes().unwrap().to_vec()).unwrap());
            }
            // Perform call
            let res = client
                .call(req)
                .await
                .map_err(InternalError::ReqwestError)?;
            // Get response data
            let status = res.status();
            let headers = res.headers().clone();
            let server = res.remote_addr().unwrap().to_string();
            let bytes = res.bytes().await.map_err(InternalError::ReqwestError)?;
            // Log it
            let mut tx = db.begin().await?;
            let new_req = NewRequest {
                client: "TODO".to_string(),
                server,
                uri,
                request_body,
                response_body: Some(String::from_utf8(bytes.to_vec()).unwrap()),
                status: status.as_u16() as i32,
            };
            let _ = crate::repository::request_repository::create_request(&mut tx, new_req).await;
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
                Err(ApiError::ClientError(ClientError::IntegrationError))
            }
        })
    }
}

/// A preconfigured HTTP client.
pub fn log_client(db: DbPool) -> impl Service<Request, Response = Response, Error = ApiError> {
    let client = reqwest::Client::new();
    ServiceBuilder::new()
        .rate_limit(1, Duration::from_secs(1))
        .layer_fn(|c| LogClient::new(c, db.clone()))
        .service(client)
}

#[cfg(test)]
mod tests {
    use super::log_client;
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
        let mut client = log_client(db);

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
