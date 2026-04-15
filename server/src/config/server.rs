use config::{Config, ConfigError, File};
use secrecy::SecretString;
use serde::Deserialize;

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub google_client_id: String,
    pub redis: RedisConfig,
    pub jwt_secret: SecretString,
    pub compress: bool,
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: Vec<String>,
    #[serde(default)]
    pub cookie_secure: bool,
}

#[must_use]
fn default_allowed_origins() -> Vec<String> {
    vec!["http://localhost:5173".to_string()]
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
            jwt_secret: SecretString::from(""),
            compress: true,
            allowed_origins: default_allowed_origins(),
            cookie_secure: false,
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

/// Newtype wrapping the JWT secret so only auth-related handlers receive it
/// via dependency injection, rather than the full `ServerConfig`.
#[derive(Clone)]
pub struct JwtSecret(SecretString);

impl JwtSecret {
    #[must_use]
    pub const fn new(secret: SecretString) -> Self {
        Self(secret)
    }

    #[must_use]
    pub fn expose_secret(&self) -> &str {
        use secrecy::ExposeSecret as _;
        self.0.expose_secret()
    }
}

/// Controls whether the `auth_token` cookie is sent with `Secure` attribute.
/// Set `false` in local development (HTTP); `true` in production (HTTPS).
#[derive(Clone)]
pub struct CookieSettings {
    pub secure: bool,
}
