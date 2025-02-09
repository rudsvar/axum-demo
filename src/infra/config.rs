//! For reading application configuration.

use axum::extract::FromRef;
use serde::Deserialize;
use std::time::Duration;

/// Application configuration.
#[derive(Clone, Debug, Deserialize, FromRef)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Server configuration.
    pub server: ServerConfig,
    /// Database configuration.
    pub database: DatabaseConfig,
    /// Jaeger configuration.
    pub logging: LoggingConfig,
    /// MQ configuration.
    pub mq: MqConfig,
    /// Email configuration.
    pub email: EmailConfig,
}

/// Server configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Server address.
    pub http_address: String,
    /// Server http port.
    pub http_port: u16,
    /// Server http port.
    pub grpc_address: String,
    /// Server https port.
    pub grpc_port: u16,
    /// Lifetime of a session in seconds.
    #[serde(with = "humantime_serde")]
    pub session_duration: Duration,
}

/// Database configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Jaeger configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    /// Logging configuration.
    pub rust_log: String,
    /// The jaeger host.
    pub jaeger_host: String,
    /// The jaeger port.
    pub jaeger_port: u16,
}

/// MQ configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MqConfig {
    /// The mq host.
    pub host: String,
    /// The mq port.
    pub port: u16,
    /// The mq username.
    pub username: String,
    /// The mq password.
    pub password: String,
}

/// Email configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailConfig {
    /// The email host.
    pub host: String,
    /// The email username.
    pub username: String,
    /// The email password.
    pub password: String,
}

/// Retrieve [`Config`] from the default configuration file.
#[tracing::instrument]
pub fn load_config() -> color_eyre::Result<Config> {
    let config = config::Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("app").separator("__"))
        .build()?
        .try_deserialize()?;
    Ok(config)
}
