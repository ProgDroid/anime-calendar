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
    #[serde(default = "default_app_base_url")]
    pub app_base_url: String,
    #[serde(default)]
    pub smtp: SmtpConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub enable_docs: bool,
}

#[must_use]
fn default_allowed_origins() -> Vec<String> {
    vec!["http://localhost:5173".to_string()]
}

fn default_app_base_url() -> String {
    "http://localhost:5173".to_string()
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
            app_base_url: default_app_base_url(),
            smtp: SmtpConfig::default(),
            metrics: MetricsConfig::default(),
            enable_docs: false,
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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SmtpConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_smtp_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: SecretString,
    #[serde(default)]
    pub from_address: String,
}

const fn default_smtp_port() -> u16 {
    587
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
    #[serde(default = "default_metrics_host")]
    pub host: String,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

const fn default_metrics_enabled() -> bool {
    true
}
fn default_metrics_host() -> String {
    "127.0.0.1".to_owned()
}
const fn default_metrics_port() -> u16 {
    9090
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
            host: default_metrics_host(),
            port: default_metrics_port(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_config_defaults_to_loopback_9090_enabled() {
        let cfg = MetricsConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 9090);
    }
}

/// Newtype wrapper for the frontend base URL — injected as `web::Data<AppBaseUrl>`.
#[derive(Clone)]
pub struct AppBaseUrl(String);

impl AppBaseUrl {
    #[must_use]
    pub const fn new(url: String) -> Self {
        Self(url)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
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
