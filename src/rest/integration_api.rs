//! Implementation of the integration API. An API that calls another service.

use super::AppState;
use crate::{
    core::item::item_repository::Item,
    infra::error::{ApiError, ApiResult, InternalError},
    integration::{http::http_client, mq::MqClient},
};
use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};
use axum_extra::{json_lines::AsResponse, response::JsonLines};
use futures::Stream;
use http::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower::Service;
use tracing::instrument;
use utoipa::ToSchema;

/// Routes for the integrations API.
pub fn integration_routes() -> Router<AppState> {
    Router::new()
        .route("/remote-items", get(remote_items))
        .route("/mq", post(post_to_mq).get(read_from_mq))
        .route("/mq2", get(stream_from_mq))
}

/// A handler for fetching items from a "remote" system.
#[utoipa::path(
    get,
    path = "/api/remote-items",
    responses(
        (status = 200, description = "Success", body = [Item]),
    )
)]
#[instrument]
pub async fn remote_items(Extension(db): Extension<PgPool>) -> Result<Json<Vec<Item>>, ApiError> {
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
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Message {
    message: String,
}

/// Post to the MQ.
#[utoipa::path(
    post,
    path = "/api/mq",
    request_body = Message,
    responses(
        (status = 201, description = "Created"),
    )
)]
#[instrument(skip(state))]
pub async fn post_to_mq(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> ApiResult<StatusCode> {
    // Get MQ client
    let conn = state.mq();
    let client: MqClient<Message> = MqClient::new(conn, "default".to_string()).await?;

    // Publish message to queue
    tracing::info!("Posting message to queue: {:?}", message);
    client.publish(&message).await?;
    Ok(StatusCode::CREATED)
}

/// Read one message from the MQ.
#[utoipa::path(
    get,
    path = "/api/mq",
    responses(
        (status = 200, description = "Success"),
    )
)]
#[instrument(skip(state))]
pub async fn read_from_mq(State(state): State<AppState>) -> ApiResult<Json<Option<Message>>> {
    // Get MQ client
    let conn = state.mq();
    let client: MqClient<Message> = MqClient::new(conn, "default".to_string()).await?;

    // Read message from queue
    let message = client.consume_one().await?;
    tracing::info!("Read message from queue: {:?}", message);

    Ok(Json(message))
}

/// Stream from the MQ.
#[utoipa::path(
    get,
    path = "/api/mq2",
    responses(
        (status = 200, description = "Success"),
    )
)]
#[instrument(skip(state))]
pub async fn stream_from_mq(
    State(state): State<AppState>,
) -> ApiResult<JsonLines<impl Stream<Item = Result<Message, ApiError>>, AsResponse>> {
    // Get MQ client
    let conn = state.mq();
    let client: MqClient<Message> = MqClient::new(conn, "default".to_string()).await?;

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
