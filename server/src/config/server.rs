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
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub stripe: StripeConfig,
    #[serde(default)]
    pub reconcile: ReconcileConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub sharing: SharingConfig,
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
            app: AppConfig::default(),
            stripe: StripeConfig::default(),
            reconcile: ReconcileConfig::default(),
            cache: CacheConfig::default(),
            limits: LimitsConfig::default(),
            sharing: SharingConfig::default(),
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

/// Top-level app config — runtime mode + the canonical frontend URL used to
/// build Stripe redirect URLs.
///
/// `environment` is the kill-switch the dev `set_subscription` CLI checks; it
/// also influences which redirect URLs we expose to Stripe.
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_environment")]
    pub environment: String,
    #[serde(default = "default_frontend_url")]
    pub frontend_url: String,
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_frontend_url() -> String {
    "http://localhost:5173".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            environment: default_environment(),
            frontend_url: default_frontend_url(),
        }
    }
}

/// Stripe config — credentials and product price ids.
///
/// All fields default to empty strings so a partial config doesn't crash startup;
/// handlers that need Stripe will surface a clear "stripe not configured" error
/// instead of failing at the deserialise layer.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct StripeConfig {
    #[serde(default)]
    pub publishable_key: String,
    #[serde(default)]
    pub secret_key: SecretString,
    #[serde(default)]
    pub webhook_secret: SecretString,
    #[serde(default)]
    pub price_id_monthly: String,
    #[serde(default)]
    pub price_id_annual: String,
}

impl StripeConfig {
    /// True when both the secret key and at least one price id are set —
    /// minimum requirement for Checkout to function.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        use secrecy::ExposeSecret as _;
        !self.secret_key.expose_secret().is_empty()
            && (!self.price_id_monthly.is_empty() || !self.price_id_annual.is_empty())
    }
}

/// Reconcile loop config — hourly safety net for the Stripe webhook path.
/// `interval_secs = 0` disables the loop (used in tests and in deployments
/// that don't want background work).
#[derive(Debug, Deserialize, Clone)]
pub struct ReconcileConfig {
    #[serde(default = "default_reconcile_interval_secs")]
    pub interval_secs: u64,
}

impl Default for ReconcileConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_reconcile_interval_secs(),
        }
    }
}

const fn default_reconcile_interval_secs() -> u64 {
    3600
}

/// TTL settings (in seconds) for the per-item / per-calendar / search /
/// export cache layers. Lower TTLs trade staleness for cache miss rate.
/// Metadata is stable; airing schedule changes faster.
#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig {
    #[serde(default = "default_cache_metadata_ttl_seconds")]
    pub metadata_ttl_seconds: u64,
    #[serde(default = "default_cache_airing_ttl_seconds")]
    pub airing_ttl_seconds: u64,
    #[serde(default = "default_cache_search_ttl_seconds")]
    pub search_ttl_seconds: u64,
    #[serde(default = "default_cache_calendar_items_ttl_seconds")]
    pub calendar_items_ttl_seconds: u64,
    #[serde(default = "default_cache_export_ttl_seconds")]
    pub export_ttl_seconds: u64,
}

const fn default_cache_metadata_ttl_seconds() -> u64 {
    86_400
} // 24h
const fn default_cache_airing_ttl_seconds() -> u64 {
    900
} // 15m
const fn default_cache_search_ttl_seconds() -> u64 {
    3_600
} // 1h
const fn default_cache_calendar_items_ttl_seconds() -> u64 {
    300
} // 5m
const fn default_cache_export_ttl_seconds() -> u64 {
    300
} // 5m

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            metadata_ttl_seconds: default_cache_metadata_ttl_seconds(),
            airing_ttl_seconds: default_cache_airing_ttl_seconds(),
            search_ttl_seconds: default_cache_search_ttl_seconds(),
            calendar_items_ttl_seconds: default_cache_calendar_items_ttl_seconds(),
            export_ttl_seconds: default_cache_export_ttl_seconds(),
        }
    }
}

/// Free-tier caps and Pro-tier ceilings. Read by the entitlement service
/// to gate calendar creation, show tracking, and per-event reminder count.
#[derive(Debug, Deserialize, Clone)]
pub struct LimitsConfig {
    #[serde(default = "default_free_calendar_limit")]
    pub free_calendar_limit: u32,
    #[serde(default = "default_free_show_cap")]
    pub free_show_cap: u32,
    #[serde(default = "default_pro_max_reminders")]
    pub pro_max_reminders: u32,
}

const fn default_free_calendar_limit() -> u32 {
    3
}
const fn default_free_show_cap() -> u32 {
    25
}
const fn default_pro_max_reminders() -> u32 {
    5
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            free_calendar_limit: default_free_calendar_limit(),
            free_show_cap: default_free_show_cap(),
            pro_max_reminders: default_pro_max_reminders(),
        }
    }
}

/// Co-editor sharing limits and timing. Read by the invitation service to
/// gate cap, expiry, rate-limit, and SSE heartbeat decisions.
#[derive(Debug, Deserialize, Clone)]
pub struct SharingConfig {
    #[serde(default = "default_editor_cap")]
    pub editor_cap: u32,
    #[serde(default = "default_invitation_expiry_days")]
    pub invitation_expiry_days: u32,
    #[serde(default = "default_invite_rate_limit_per_hour")]
    pub invite_rate_limit_per_hour: u32,
    #[serde(default = "default_presence_ttl_seconds")]
    pub presence_ttl_seconds: u32,
    #[serde(default = "default_presence_heartbeat_seconds")]
    pub presence_heartbeat_seconds: u32,
    #[serde(default = "default_sse_heartbeat_seconds")]
    pub sse_heartbeat_seconds: u32,
    #[serde(default = "default_sse_max_connections_per_user")]
    pub sse_max_connections_per_user: u32,
}

const fn default_editor_cap() -> u32 {
    5
}

const fn default_invitation_expiry_days() -> u32 {
    7
}

const fn default_invite_rate_limit_per_hour() -> u32 {
    20
}

const fn default_presence_ttl_seconds() -> u32 {
    60
}

const fn default_presence_heartbeat_seconds() -> u32 {
    30
}

const fn default_sse_heartbeat_seconds() -> u32 {
    25
}

const fn default_sse_max_connections_per_user() -> u32 {
    8
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            editor_cap: default_editor_cap(),
            invitation_expiry_days: default_invitation_expiry_days(),
            invite_rate_limit_per_hour: default_invite_rate_limit_per_hour(),
            presence_ttl_seconds: default_presence_ttl_seconds(),
            presence_heartbeat_seconds: default_presence_heartbeat_seconds(),
            sse_heartbeat_seconds: default_sse_heartbeat_seconds(),
            sse_max_connections_per_user: default_sse_max_connections_per_user(),
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

    #[test]
    fn validate_rejects_production_without_cookie_secure() {
        let cfg = Server {
            app: AppConfig {
                environment: "production".to_string(),
                ..AppConfig::default()
            },
            cookie_secure: false,
            ..Server::default()
        };
        let result = cfg.validate();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("cookie_secure"),
            "error message should mention cookie_secure: {msg}"
        );
        assert!(
            msg.contains("production"),
            "error message should mention production: {msg}"
        );
    }

    #[test]
    fn validate_accepts_production_with_cookie_secure() {
        let cfg = Server {
            app: AppConfig {
                environment: "production".to_string(),
                ..AppConfig::default()
            },
            cookie_secure: true,
            ..Server::default()
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_accepts_development_without_cookie_secure() {
        let cfg = Server {
            app: AppConfig {
                environment: "development".to_string(),
                ..AppConfig::default()
            },
            cookie_secure: false,
            ..Server::default()
        };
        assert!(cfg.validate().is_ok());
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

    /// Reject configurations that would be unsafe in production. Currently:
    /// - `cookie_secure = false` while `app.environment = "production"` would
    ///   issue auth/refresh cookies without the Secure flag, leaking them on
    ///   any non-HTTPS hop. The server refuses to start rather than degrade
    ///   silently.
    ///
    /// # Errors
    /// Returns a descriptive `String` if the configuration is rejected.
    pub fn validate(&self) -> Result<(), String> {
        if self.app.environment == "production" && !self.cookie_secure {
            return Err("config rejected: cookie_secure must be true in production \
                 (otherwise auth/refresh cookies are issued without the Secure \
                 flag and may be exposed on non-HTTPS hops)"
                .to_string());
        }
        Ok(())
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
