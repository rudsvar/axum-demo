//! Implementation of the integration API. An API that calls another service.

use crate::{
    core::item::item_repository::Item,
    infra::{
        error::{ApiError, ApiResult, InternalError},
        extract::Json,
        state::AppState,
    },
    integration::{
        http::http_client,
        mq::{MqClient, MqPool},
    },
};
use aide::axum::{
    routing::{get, post},
    ApiRouter,
};
use axum::{extract::State, Extension};
use axum_extra::{json_lines::AsResponse, response::JsonLines};
use futures::Stream;
use http::{Method, StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower::Service;
use tracing::instrument;

/// Routes for the integrations API.
pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route("/remote-items", get(remote_items))
        .api_route("/mq", post(post_to_mq).get(read_from_mq))
        .route("/mq2", axum::routing::get(stream_from_mq))
}

/// A handler for fetching items from a "remote" system.
#[instrument]
pub async fn remote_items(Extension(db): Extension<PgPool>) -> ApiResult<Json<Vec<Item>>> {
    let mut client = http_client(db);
    let req = reqwest::Request::new(
        Method::GET,
        "http://localhost:8080/api/items".parse().unwrap(),
    );
    let res = client.call(req).await?;
    let res: Vec<Item> = res.json().await.map_err(InternalError::from)?;
    Ok(Json(res))
}

/// The MQ message format.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Message {
    message: String,
}

/// Post to the MQ.
#[instrument(skip(pool))]
pub async fn post_to_mq(
    State(pool): State<MqPool>,
    Json(message): Json<Message>,
) -> ApiResult<StatusCode> {
    // Get MQ client
    let conn = pool.get().await?;
    let client: MqClient<Message> = MqClient::new(&conn, "default".to_string()).await?;

    // Publish message to queue
    tracing::info!("Posting message to queue: {:?}", message);
    client.publish(&message).await?;
    Ok(StatusCode::CREATED)
}

/// Read one message from the MQ.
#[instrument(skip(pool))]
pub async fn read_from_mq(State(pool): State<MqPool>) -> ApiResult<Json<Option<Message>>> {
    // Get MQ client
    let conn = pool.get().await?;
    let client: MqClient<Message> = MqClient::new(&conn, "default".to_string()).await?;

    // Read message from queue
    let message = client.consume_one().await?;
    tracing::info!("Read message from queue: {:?}", message);

    Ok(Json(message))
}

/// Stream from the MQ.
#[instrument(skip(pool))]
pub async fn stream_from_mq(
    State(pool): State<MqPool>,
) -> ApiResult<JsonLines<impl Stream<Item = Result<Message, ApiError>>, AsResponse>> {
    // Get MQ client
    let conn = pool.get().await?;
    let client: MqClient<Message> = MqClient::new(&conn, "default".to_string()).await?;

    // Read message from queue
    let stream = client.consume();

    Ok(JsonLines::new(stream))
}

#[cfg(test)]
mod tests {
    use crate::{infra::database::DbPool, rest::integration_api::remote_items};
    use axum::Extension;

    #[sqlx::test]
    async fn it_works(db: DbPool) {
        let response = remote_items(Extension(db)).await;
        assert!(response.is_err())
    }
}
