//! Utilities for integrating with a message queue.

use crate::infra::{config::MqConfig, error::InternalError};
use futures::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    publisher_confirm::Confirmation,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, Consumer, Queue,
};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

/// A client for simplifying interacting with the message queue.
#[derive(Clone, Debug)]
pub struct MqClient<T> {
    channel: Channel,
    queue: Queue,
    ty: PhantomData<T>,
}

impl<T> MqClient<T> {
    /// Creates a new client.
    pub async fn new(connection: &Connection, queue: String) -> Result<Self, InternalError> {
        let channel = connection.create_channel().await?;
        let queue = queue_declare(&channel, &queue).await?;
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
        publish(&self.channel, self.queue.name().as_str(), message).await
    }

    /// Consumes a message from the message queue.
    pub async fn consume_one(&self) -> Result<Option<T>, InternalError>
    where
        T: DeserializeOwned,
    {
        consume_one(&self.channel, self.queue.name().as_str()).await
    }

    /// Consumes multiple messages from the message queue.
    pub async fn consume(&self) -> Result<Consumer, InternalError>
    where
        T: DeserializeOwned,
    {
        consume::<T>(&self.channel, self.queue.name().as_str()).await
    }
}

/// Establishes a connection to the message queue.
pub async fn connect(config: &MqConfig) -> Connection {
    let addr = config.connection_string();
    let conn = Connection::connect(&addr, ConnectionProperties::default())
        .await
        .unwrap();
    conn
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
pub async fn consume<T: DeserializeOwned>(
    channel: &Channel,
    queue: &str,
) -> Result<Consumer, InternalError> {
    let consumer = channel
        .basic_consume(
            queue,
            "consume",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    Ok(consumer)
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
        let conn = connect(&config.mq).await;
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
        let conn = connect(&config.mq).await;
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
        let conn = connect(&config.mq).await;
        let channel = conn.create_channel().await.unwrap();
        consume_one::<Message>(&channel, "hello").await.unwrap();
    }
}
