//! For reading application configuration.

use axum::extract::FromRef;
use serde::Deserialize;

/// Application configuration.
#[derive(Clone, Debug, Deserialize, FromRef)]
pub struct Config {
    /// Server configuration.
    pub server: ServerConfig,
    /// Database configuration.
    pub database: DatabaseConfig,
    /// Message queue configuration.
    pub mq: MqConfig,
    /// Email configuration.
    pub email: EmailConfig,
}

/// Server configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    /// Server address.
    pub http_address: String,
    /// Server http port.
    pub http_port: u16,
    /// Server http port.
    pub grpc_address: String,
    /// Server https port.
    pub grpc_port: u16,
}

/// Database configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    /// The database username.
    pub username: String,
    /// The database password.
    pub password: String,
    /// The database port.
    pub port: u16,
    /// The database name.
    pub database_name: String,
    /// The database host.
    pub host: String,
}

/// Message queue configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct MqConfig {
    /// The message queue username.
    pub username: String,
    /// The message queue password.
    pub password: String,
    /// The message queue host.
    pub host: String,
    /// The message queue port.
    pub port: u16,
}

impl MqConfig {
    /// Constructs a connection string.
    pub fn connection_string(&self) -> String {
        format!(
            "amqp://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

/// Email configuration
#[derive(Clone, Debug, Deserialize)]
pub struct EmailConfig {
    /// The email username.
    pub username: String,
    /// The email password.
    pub password: String,
    /// The email host.
    pub host: String,
}

/// Retrieve [`Config`] from the default configuration file.
#[tracing::instrument]
pub fn load_config() -> anyhow::Result<Config> {
    let config = config::Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("app").separator("__"))
        .build()?
        .try_deserialize()?;
    Ok(config)
}
