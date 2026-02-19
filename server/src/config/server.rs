use config::{Config, ConfigError, File};
use serde::Deserialize;

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub google_client_id: String,
    pub redis: RedisConfig,
    pub jwt_secret: String, // TODO secret?
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub db: u8,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            log_level: "debug".to_string(),
            google_client_id: String::new(),
            redis: RedisConfig::default(),
            jwt_secret: String::new(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            password: String::new(),
            db: 0,
        }
    }
}

impl Server {
    /// # Errors
    /// Returns `ConfigError` if config file is invalid or not found
    pub fn new() -> std::result::Result<Self, ConfigError> {
        let server_config = Config::builder()
            .add_source(File::with_name(CONFIG_FILE))
            .build()?;

        server_config.try_deserialize()
    }
}
