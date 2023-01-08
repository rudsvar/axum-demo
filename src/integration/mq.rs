//! Utilities for integrating with a message queue.

use crate::infra::{
    config::MqConfig,
    error::{ApiError, InternalError},
};
use async_stream::try_stream;
use deadpool_lapin::{Manager, Pool};
use futures::{Stream, StreamExt, TryStreamExt};
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    publisher_confirm::Confirmation,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, Queue,
};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

/// A common MQ pool type.
pub type MqPool = deadpool_lapin::Pool;

/// A client for simplifying interacting with the message queue.
#[derive(Clone, Debug)]
pub struct MqClient<T> {
    channel: Channel,
    queue: String,
    ty: PhantomData<T>,
}

impl<T> MqClient<T> {
    /// Creates a new client.
    pub async fn new(connection: &Connection, queue: String) -> Result<Self, InternalError> {
        let channel = connection.create_channel().await?;
        queue_declare(&channel, &queue).await?;
        Ok(Self {
            channel,
            queue,
            ty: PhantomData,
        })
    }

    /// Publishes a message to the message queue.
    pub async fn publish(&self, message: &T) -> Result<Confirmation, InternalError>
    where
        T: Serialize,
    {
        publish(&self.channel, &self.queue, message).await
    }

    /// Consumes a message from the message queue.
    pub async fn consume_one(&self) -> Result<Option<T>, InternalError>
    where
        T: DeserializeOwned,
    {
        consume_one(&self.channel, &self.queue).await
    }

    /// Consumes multiple messages from the message queue.
    pub fn consume(self) -> impl Stream<Item = Result<T, ApiError>>
    where
        T: DeserializeOwned,
    {
        consume(self.channel, self.queue)
    }
}

/// Establishes a connection to the message queue.
pub async fn init_mq(config: &MqConfig) -> Result<Pool, InternalError> {
    let addr = config.connection_string();
    let manager = Manager::new(addr, ConnectionProperties::default());
    Pool::builder(manager)
        .build()
        .map_err(|e| InternalError::Other(e.to_string()))
}

/// Declares a new queue.
pub async fn queue_declare(channel: &Channel, queue: &str) -> Result<Queue, InternalError> {
    let queue = channel
        .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
        .await?;
    tracing::info!("Declared queue {}", queue.name());
    Ok(queue)
}

/// Publishes a message on a queue.
pub async fn publish<T: Serialize>(
    channel: &Channel,
    queue: &str,
    message: &T,
) -> Result<Confirmation, InternalError> {
    let serialized = serde_json::to_vec(message)?;
    let confirm = channel
        .basic_publish(
            "",
            queue,
            BasicPublishOptions::default(),
            &serialized,
            BasicProperties::default(),
        )
        .await?
        .await?;
    Ok(confirm)
}

/// Consumes a single message from a queue.
pub async fn consume_one<T: DeserializeOwned>(
    channel: &Channel,
    queue: &str,
) -> Result<Option<T>, InternalError> {
    let mut consumer = channel
        .basic_consume(
            queue,
            "consume_one",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    if let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        let data = serde_json::from_slice(&delivery.data)?;
        delivery.ack(BasicAckOptions::default()).await?;
        return Ok(Some(data));
    }
    Ok(None)
}

/// Consumes messages from a queue.
pub fn consume<T: DeserializeOwned>(
    channel: Channel,
    queue: String,
) -> impl Stream<Item = Result<T, ApiError>> {
    let stream = try_stream! {
        // Create consumer
        let mut consumer = channel
            .basic_consume(
                &queue,
                "consume",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        // Yield values from consumer after deserializing
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery?;
            let data: T = serde_json::from_slice(&delivery.data)?;
            delivery.ack(BasicAckOptions::default()).await?;
            yield data;
        }
    };
    stream.map_err(ApiError::InternalError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::load_config;
    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Message {
        name: String,
        age: i32,
    }

    #[tokio::test]
    #[ignore = "requires mq"]
    async fn send_and_recv() {
        let config = load_config().unwrap();
        let conn = init_mq(&config.mq).await.unwrap();
        let conn = conn.get().await.unwrap();
        let sender = conn.create_channel().await.unwrap();
        let receiver = conn.create_channel().await.unwrap();
        let message = Message {
            name: "foo".to_string(),
            age: 212,
        };
        let queue = "hello";
        queue_declare(&sender, queue).await.unwrap();
        tokio::spawn(async move {
            publish(&sender, queue, &message).await.unwrap();
        });
        consume_one::<Message>(&receiver, queue).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires mq"]
    async fn send_test() {
        let config = load_config().unwrap();
        let pool = init_mq(&config.mq).await.unwrap();
        let conn = pool.get().await.unwrap();
        let sender = conn.create_channel().await.unwrap();
        let message = Message {
            name: "foo".to_string(),
            age: 212,
        };
        let queue = "hello";
        queue_declare(&sender, queue).await.unwrap();
        publish(&sender, queue, &message).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires mq"]
    async fn recv_test() {
        let config = load_config().unwrap();
        let pool = init_mq(&config.mq).await.unwrap();
        let conn = pool.get().await.unwrap();
        let channel = conn.create_channel().await.unwrap();
        consume_one::<Message>(&channel, "hello").await.unwrap();
    }
}
