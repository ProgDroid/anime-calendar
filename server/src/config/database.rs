use config::{Config, ConfigError, File};
use secrecy::SecretString;
use serde::Deserialize;

const DATABASE_FILE: &str = "database.toml";
const ENV_PREFIX: &str = "DATABASE";

#[allow(clippy::struct_field_names)]
#[derive(Deserialize, Clone)]
pub struct Database {
    pub user: String,
    pub pass: SecretString,
    pub host: String,
    pub port: String,
    pub database: String,
    /// Full connection string (a managed provider's DSN). When set, the
    /// individual fields above are ignored.
    #[serde(default)]
    pub url: Option<SecretString>,
    /// Upper bound on connections in the (single, shared) pool.
    ///
    /// This is the whole connection budget for one process, so the arithmetic
    /// a managed Postgres must satisfy is:
    ///
    /// ```text
    /// max_connections × (Cloud Run max-instances) + headroom  ≤  server limit
    /// ```
    ///
    /// where headroom covers migrations, the `set_subscription` CLI and any
    /// interactive `psql`. A `db-f1-micro` allows roughly 25, so the default of
    /// 5 leaves room for four instances plus five spare connections.
    ///
    /// Raising this is a real decision, not a tuning knob — read the formula
    /// above and check it against the instance you actually provisioned.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

/// Deliberately small. sqlx's own default is 10 *per pool*, which was the
/// wrong unit here: before the pools were unified this process built fifteen
/// of them, for a ceiling of 150 connections against a `db-f1-micro` limit of
/// about 25.
const fn default_max_connections() -> u32 {
    5
}

impl Database {
    /// Load from `database.toml`, then let environment variables override it.
    /// Cloud Run ships no config file, so the env layer must work standalone —
    /// hence `required(false)`.
    ///
    /// Env vars are read with a `DATABASE__` prefix: `DATABASE__URL` sets
    /// `url`, `DATABASE__HOST` sets `host`. The prefix is load-bearing, for
    /// two reasons:
    ///
    /// 1. Unprefixed, `DATABASE__URL` would split on `__` into `database.url`
    ///    — a map nested under the `database` *string* field — and fail to
    ///    deserialize rather than setting `url`.
    /// 2. These field names are ordinary words. `USER`, `HOST`, `PORT` and
    ///    `PASS` all exist in common shells and CI runners, and unprefixed
    ///    they would silently override `database.toml`. The `config` crate
    ///    skips any key not matching the prefix, so prefixing closes that off.
    ///
    /// # Errors
    /// Returns `ConfigError` if config is invalid
    pub fn new() -> std::result::Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name(DATABASE_FILE).required(false))
            .add_source(Self::env_source())
            .build()?
            .try_deserialize()
    }

    /// The env layer, shared by `new` and the tests so they exercise the real
    /// prefix and separator rather than a copy that could drift.
    fn env_source() -> config::Environment {
        config::Environment::with_prefix(ENV_PREFIX)
            .separator("__")
            .try_parsing(true)
    }

    /// Env-only load for tests: deliberately skips the file source so results
    /// do not depend on whether a `database.toml` exists on the machine
    /// running the suite.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    /// Every field the struct requires, so the env layer is exercised standalone.
    fn required() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DATABASE__USER", "cfg_user"),
            ("DATABASE__PASS", "cfg_pass"),
            ("DATABASE__HOST", "cfg_host"),
            ("DATABASE__PORT", "5432"),
            ("DATABASE__DATABASE", "cfg_db"),
        ]
    }

    /// Cloud Run mounts no config file — env alone has to be sufficient.
    #[test]
    fn loads_from_environment_with_no_config_file() {
        let cfg = Database::from_env_map(&required()).expect("env-only load should succeed");
        assert_eq!(cfg.user, "cfg_user");
        assert_eq!(cfg.host, "cfg_host");
        assert_eq!(cfg.database, "cfg_db");
        assert!(cfg.url.is_none());
    }

    /// The deploy workflow injects `DATABASE__URL`. Unprefixed it would split on
    /// `__` into `database.url` and fail to deserialize instead of landing here.
    #[test]
    fn database_url_env_var_populates_url_field() {
        let mut vars = required();
        vars.push(("DATABASE__URL", "postgres://u:p@ep-x.neon.tech/neondb"));
        let cfg = Database::from_env_map(&vars).expect("load should succeed");
        assert_eq!(
            cfg.url.as_ref().map(ExposeSecret::expose_secret),
            Some("postgres://u:p@ep-x.neon.tech/neondb")
        );
    }

    /// Locks in the reason `env_source` is prefixed. With the unprefixed
    /// source the original plan specified, `DATABASE__URL` splits on `__` into
    /// `database` -> `url`, i.e. a map landing on the `database: String`
    /// field, and the load fails instead of populating `url`. If this ever
    /// starts succeeding, the prefix decision needs revisiting.
    #[test]
    fn unprefixed_source_misroutes_database_url() {
        let map: config::Map<String, String> = required()
            .into_iter()
            .chain([("DATABASE__URL", "postgres://u:p@host/db")])
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let result: Result<Database, _> = Config::builder()
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true)
                    .source(Some(map)),
            )
            .build()
            .and_then(Config::try_deserialize);
        assert!(
            result.is_err(),
            "unprefixed DATABASE__URL should collide with the `database` field, got {:?}",
            result.map(|d| d.database)
        );
    }

    /// `USER`, `HOST`, `PORT` and `PASS` exist in ordinary shells and CI
    /// runners. Without the prefix they would silently override the configured
    /// credentials; the prefix makes the loader skip them outright.
    #[test]
    fn unprefixed_ambient_vars_are_ignored() {
        let mut vars = required();
        vars.extend([
            ("USER", "ambient_user"),
            ("HOST", "ambient_host"),
            ("PASS", "ambient_pass"),
            ("DATABASE", "ambient_db"),
        ]);
        let cfg = Database::from_env_map(&vars).expect("load should succeed");
        assert_eq!(cfg.user, "cfg_user", "ambient USER must not win");
        assert_eq!(cfg.host, "cfg_host", "ambient HOST must not win");
        assert_eq!(cfg.pass.expose_secret(), "cfg_pass");
        assert_eq!(cfg.database, "cfg_db");
    }
}
