use config::{Config, ConfigError, File};
use secrecy::SecretString;
use serde::Deserialize;

const CONFIG_FILE: &str = "config.toml";

// Config structs mirror the TOML surface 1:1; independent feature toggles are
// genuinely independent bools, not a hidden state machine.
#[allow(clippy::struct_excessive_bools)]
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
    /// When the server sits behind a trusted reverse proxy (nginx in the
    /// shipped topology), every TCP connection arrives from the proxy's IP,
    /// so per-peer rate limiting collapses into one shared bucket. Set this
    /// to `true` to key the rate limiter on the rightmost `X-Forwarded-For`
    /// entry (the hop appended by the proxy itself) instead. Leave `false`
    /// when clients connect directly — a spoofed header must never pick the
    /// bucket.
    #[serde(default)]
    pub trust_proxy_header: bool,
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
    /// Shared secret injected by Cloudflare via the `X-CF-Origin-Secret`
    /// header. When set, requests arriving without it are rejected — this is
    /// what stops traffic bypassing Cloudflare and hitting Cloud Run directly.
    #[serde(default)]
    pub cf_origin_secret: Option<String>,
    /// Cookie domain (e.g. `.yourdomain.com`). When set, auth cookies carry
    /// `Domain=<value>` so they are shared across subdomains. Leave unset in
    /// local dev, where host-only cookies are correct.
    #[serde(default)]
    pub cookie_domain: Option<String>,
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
    /// Full Redis URL (e.g. `rediss://...` for Upstash TLS). When set, the
    /// individual host/port/password/db fields above are ignored.
    #[serde(default)]
    pub url: Option<String>,
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
            trust_proxy_header: false,
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
            cf_origin_secret: None,
            cookie_domain: None,
        }
    }
}

impl RedisConfig {
    /// The URL every Redis consumer should connect with: the explicit `url`
    /// when set (Upstash and friends hand out a full `rediss://` string),
    /// otherwise one assembled from the individual fields.
    ///
    /// Returned as a `SecretString` because it embeds the password. Resolving
    /// this in one place matters: the cache, the Pub/Sub client and the
    /// presence service each open their own connection, and if only some of
    /// them honoured `url` the rest would quietly dial localhost.
    #[must_use]
    pub fn connection_url(&self) -> SecretString {
        self.url.clone().map_or_else(
            || {
                if self.password.is_empty() {
                    SecretString::from(format!("redis://{}:{}", self.host, self.port))
                } else {
                    SecretString::from(format!(
                        "redis://:{}@{}:{}",
                        self.password, self.host, self.port
                    ))
                }
            },
            SecretString::from,
        )
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            password: String::new(),
            db: 0,
            url: None,
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

    /// Every field `Server` requires, so the env layer is exercised standalone.
    fn required() -> Vec<(&'static str, &'static str)> {
        vec![
            ("HOST", "0.0.0.0"),
            ("PORT", "8080"),
            ("LOG_LEVEL", "info"),
            ("GOOGLE_CLIENT_ID", "cid.apps.googleusercontent.com"),
            ("JWT_SECRET", "s3cret"),
            ("COMPRESS", "true"),
            ("REDIS__HOST", "127.0.0.1"),
            ("REDIS__PORT", "6379"),
            ("REDIS__PASSWORD", ""),
            ("REDIS__DB", "0"),
        ]
    }

    /// The whole point of Task 2: Cloud Run mounts no config.toml, so the env
    /// layer alone has to produce a usable Server config.
    #[test]
    fn loads_from_environment_with_no_config_file() {
        let cfg = Server::from_env_map(&required()).expect("env-only load should succeed");
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.compress, "try_parsing should coerce \"true\" to a bool");
    }

    /// `__` is the nesting separator; a single underscore is part of the name.
    /// If that ever flipped, `LOG_LEVEL` would become `log.level` and silently
    /// stop applying.
    #[test]
    fn single_underscore_is_part_of_the_name_not_a_separator() {
        let cfg = Server::from_env_map(&required()).expect("load should succeed");
        assert_eq!(cfg.log_level, "info");
    }

    /// The deploy workflow injects `REDIS__URL` for Upstash TLS; it must land on
    /// the nested redis config, not a top-level key.
    #[test]
    fn redis_url_env_var_maps_to_nested_redis_url() {
        let mut vars = required();
        vars.push(("REDIS__URL", "rediss://default:tok@eu1.upstash.io:6379"));
        let cfg = Server::from_env_map(&vars).expect("load should succeed");
        assert_eq!(
            cfg.redis.url.as_deref(),
            Some("rediss://default:tok@eu1.upstash.io:6379")
        );
    }

    /// The deploy workflow injects `ALLOWED_ORIGINS` as a single scalar. Before
    /// `list_separator` was configured this failed the whole load with
    /// *"invalid type: string ..., expected a sequence"* — the container would
    /// not start, and nothing in the suite noticed because every other test
    /// let `allowed_origins` fall back to its serde default.
    #[test]
    fn allowed_origins_accepts_a_single_env_value() {
        let mut vars = required();
        vars.push(("ALLOWED_ORIGINS", "https://staging.example.com"));
        let cfg = Server::from_env_map(&vars).expect("single-origin env load should succeed");
        assert_eq!(cfg.allowed_origins, vec!["https://staging.example.com"]);
    }

    /// Production fronts both the apex and the `www` host.
    #[test]
    fn allowed_origins_splits_on_commas() {
        let mut vars = required();
        vars.push((
            "ALLOWED_ORIGINS",
            "https://example.com,https://www.example.com",
        ));
        let cfg = Server::from_env_map(&vars).expect("multi-origin env load should succeed");
        assert_eq!(
            cfg.allowed_origins,
            vec!["https://example.com", "https://www.example.com"]
        );
    }

    /// The split is scoped to `allowed_origins` on purpose: a bare
    /// `list_separator` turns every string field into a list, so a secret that
    /// happens to contain a comma would be silently truncated to its first
    /// segment.
    #[test]
    fn a_comma_in_a_scalar_field_is_not_split() {
        use secrecy::ExposeSecret as _;
        let mut vars = required();
        vars.push(("JWT_SECRET", "abc,def"));
        let cfg = Server::from_env_map(&vars).expect("load should succeed");
        assert_eq!(cfg.jwt_secret.expose_secret(), "abc,def");
    }

    /// Both new fields default to None so existing deployments are unaffected.
    #[test]
    fn cf_origin_secret_and_cookie_domain_default_to_none_and_accept_env() {
        let cfg = Server::from_env_map(&required()).expect("load should succeed");
        assert!(cfg.cf_origin_secret.is_none());
        assert!(cfg.cookie_domain.is_none());

        let mut vars = required();
        vars.push(("CF_ORIGIN_SECRET", "cf-shared-secret"));
        vars.push(("COOKIE_DOMAIN", ".example.com"));
        let cfg = Server::from_env_map(&vars).expect("load should succeed");
        assert_eq!(cfg.cf_origin_secret.as_deref(), Some("cf-shared-secret"));
        assert_eq!(cfg.cookie_domain.as_deref(), Some(".example.com"));
    }

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
    /// Load configuration from `config.toml`, then let environment variables
    /// override it. Cloud Run ships no config file, so the env layer has to
    /// work standalone — hence `required(false)` on the file source.
    ///
    /// Env vars are read **unprefixed** with `__` as the nesting separator:
    /// `PORT` sets `port`, `LOG_LEVEL` sets `log_level` (a single underscore
    /// is part of the name, not a separator), and `REDIS__URL` sets
    /// `redis.url`. Unprefixed is deliberate here: Cloud Run injects `PORT`
    /// itself and the container must honour it. The trade-off is that an
    /// ambient variable sharing a field's name (`HOST`, `COMPRESS`) will also
    /// be picked up — see `Database::new`, which is prefixed precisely
    /// because its field names are far more collision-prone.
    ///
    /// # Errors
    /// Returns `ConfigError` if config is invalid
    pub fn new() -> std::result::Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name(CONFIG_FILE).required(false))
            .add_source(Self::env_source())
            .build()?
            .try_deserialize()
    }

    /// The env layer, shared by `new` and the tests so they exercise the real
    /// separator rather than a copy that could drift.
    ///
    /// `allowed_origins` is a `Vec<String>`, and an env var is always a scalar:
    /// without `list_separator` the load fails outright with *"invalid type:
    /// string ..., expected a sequence"*, so a container told
    /// `ALLOWED_ORIGINS=https://example.com` refuses to start. The split is
    /// scoped with `with_list_parse_key` rather than applied globally —
    /// a bare `list_separator` turns **every** string field into a list, which
    /// would break `jwt_secret`, `log_level` and any other value containing a
    /// comma.
    fn env_source() -> config::Environment {
        config::Environment::default()
            .separator("__")
            .try_parsing(true)
            .list_separator(",")
            .with_list_parse_key("allowed_origins")
    }

    /// Env-only load for tests: deliberately skips the file source so results
    /// do not depend on whether a `config.toml` exists on the machine running
    /// the suite.
    #[cfg(test)]
    fn from_env_map(vars: &[(&str, &str)]) -> std::result::Result<Self, ConfigError> {
        let map: config::Map<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::builder()
            .add_source(Self::env_source().source(Some(map)))
            .build()?
            .try_deserialize()
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
    /// When set, cookies are issued with `Domain=<value>` so they are valid
    /// across subdomains (app + api). `None` yields host-only cookies.
    pub domain: Option<String>,
}
