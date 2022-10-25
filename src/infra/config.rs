//! For reading application configuration.

use serde::Deserialize;

/// Application settings.
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Server settings.
    pub server: ServerConfig,
    /// Database settings.
    pub database: DatabaseConfig,
}

/// Server settings.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    /// Server address.
    pub address: String,
    /// Server http port.
    pub http_port: u16,
    /// Server http port.
    pub grpc_address: String,
    /// Server https port.
    pub grpc_port: u16,
}

/// Database settings.
#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    /// The database username.
    pub username: String,
    /// The database password.
    pub password: String,
    /// The database port.
    pub port: u16,
    /// The database host.
    pub host: String,
    /// The database name.
    pub database_name: String,
}

/// Retrieve [`Config`] from the default configuration file.
#[tracing::instrument]
pub fn load_config() -> anyhow::Result<Config> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("app").separator("_"))
        .build()?
        .try_deserialize()?;
    Ok(settings)
}
