//! Types and functions for storing and loading requests from the database.

use crate::infra::{database::Tx, error::ApiResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// A new request.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewRequest {
    /// The sender of the request.
    pub client: Option<String>,
    /// The receiver of the request.
    pub server: Option<String>,
    /// The request URI.
    pub uri: String,
    /// The request body.
    pub request_body: Option<String>,
    /// The response body.
    pub response_body: Option<String>,
    /// The response status.
    pub status: i32,
}

/// A request.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// A unique id for this request.
    pub id: i32,
    /// The sender of the request.
    pub client: Option<String>,
    /// The receiver of the request.
    pub server: Option<String>,
    /// The request URI.
    pub uri: String,
    /// The request body.
    pub request_body: Option<String>,
    /// The response body.
    pub response_body: Option<String>,
    /// A timestamp of when the request was made.
    pub timestamp: DateTime<Utc>,
    /// The response status.
    pub status: i32,
}

/// Creates a new item.
#[instrument(skip(tx))]
pub async fn create_request(tx: &mut Tx, new_req: NewRequest) -> ApiResult<Request> {
    tracing::info!("Creating request {:?}", new_req);
    let req = sqlx::query_as!(
        Request,
        r#"
        INSERT INTO requests (client, server, uri, request_body, response_body, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
        new_req.client,
        new_req.server,
        new_req.uri,
        new_req.request_body,
        new_req.response_body,
        new_req.status
    )
    .fetch_one(tx)
    .await?;
    tracing::info!("Created req {:?}", req);
    Ok(req)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_works(db: PgPool) {
        tracing_subscriber::fmt().init();
        let mut tx = db.begin().await.unwrap();
        let req = create_request(
            &mut tx,
            NewRequest {
                client: "127.0.0.1".to_string(),
                server: "127.0.0.1".to_string(),
                uri: "/foo/bar".to_string(),
                request_body: None,
                response_body: Some(r#"{"foo": "bar"}"#.to_string()),
                status: 200,
            },
        )
        .await
        .unwrap();

        assert_eq!(req.uri, "/foo/bar");
    }
}
