use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{ServerResult, config::database::Database as Config};

/// The DSN to connect with.
///
/// `config.url` when set, otherwise one assembled from the individual fields.
/// A managed provider hands out a full connection string, often carrying
/// required query parameters such as `sslmode=require` that a field-by-field
/// rebuild would silently drop.
///
/// Returned as a `SecretString` because it embeds the password.
///
/// Public because the `set_subscription` CLI has to resolve the DSN exactly as
/// the server does. It previously assembled its own from the individual fields
/// and silently ignored `url`, which meant it could not reach a managed
/// Postgres at all — the one place you most want to be sure you are talking to
/// the database you think you are.
#[must_use]
pub fn connection_string(config: &Config) -> SecretString {
    config.url.as_ref().map_or_else(
        || {
            SecretString::from(format!(
                "postgres://{}:{}@{}:{}/{}",
                config.user,
                config.pass.expose_secret(),
                config.host,
                config.port,
                config.database
            ))
        },
        Clone::clone,
    )
}

/// A human-readable description of where a DSN points — host and database
/// only, never the password.
///
/// Used to print the target before a mutation, so an operator can see which
/// database they are about to change. `APP_ENV` states *intent*; this states
/// what is actually on the other end of the socket.
#[must_use]
pub fn describe_target(config: &Config) -> String {
    config.url.as_ref().map_or_else(
        || format!("{}:{}/{}", config.host, config.port, config.database),
        |url| {
            let raw = url.expose_secret();
            // Strip any `scheme://user:pass@` prefix; keep the rest.
            let after_scheme = raw.split_once("://").map_or(raw, |(_, rest)| rest);
            let host_and_path = after_scheme
                .rsplit_once('@')
                .map_or(after_scheme, |(_, rest)| rest);
            host_and_path.to_owned()
        },
    )
}

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    /// Build the process's connection pool.
    ///
    /// **Call this once.** `Database` is `Clone` and a cloned `PgPool` shares
    /// the same underlying pool, so every mapper takes a clone rather than
    /// constructing its own. That is load-bearing, not tidiness: this used to
    /// be called fifteen times from `main.rs`, producing fifteen independent
    /// pools at sqlx's default of 10 connections each — a ceiling of 150 per
    /// instance, multiplied again by Cloud Run's `max-instances`, against a
    /// `db-f1-micro` limit of roughly 25.
    ///
    /// `max_connections` is set explicitly because sqlx's default is
    /// documented as suiting "testing or light-duty applications"; see
    /// [`crate::config::database::Database::max_connections`] for the
    /// arithmetic a managed Postgres has to satisfy.
    ///
    /// # Errors
    /// Returns an error if the connection to the database fails
    pub async fn new(config: Config) -> ServerResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(connection_string(&config).expose_secret())
            .await?;

        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    fn config() -> Config {
        Config {
            user: "u".to_owned(),
            pass: SecretString::from("p"),
            host: "h".to_owned(),
            port: "5432".to_owned(),
            database: "d".to_owned(),
            url: None,
            max_connections: 5,
        }
    }

    #[test]
    fn builds_dsn_from_individual_fields_when_url_is_unset() {
        assert_eq!(
            connection_string(&config()).expose_secret(),
            "postgres://u:p@h:5432/d"
        );
    }

    /// The whole point of unifying the pools: the connection budget for a
    /// process must be one number a human can check against the Postgres
    /// instance's limit, not sqlx's per-pool default multiplied by however
    /// many pools happen to get constructed.
    ///
    /// This asserts the *default*; that `Database::new` actually applies it
    /// needs a live server, so it is covered by the mapper integration tests
    /// rather than here.
    #[test]
    fn max_connections_defaults_low_enough_for_a_small_managed_instance() {
        let cfg: Config = ::config::Config::builder()
            .add_source(::config::File::from_str(
                r#"{"user":"u","pass":"p","host":"h","port":"5432","database":"d"}"#,
                ::config::FileFormat::Json,
            ))
            .build()
            .expect("builder should succeed")
            .try_deserialize()
            .expect("config without max_connections should fall back to the default");

        assert_eq!(cfg.max_connections, 5);

        // db-f1-micro allows roughly 25. Four Cloud Run instances at the
        // default must still leave headroom for migrations and a psql session.
        let max_instances = 4;
        assert!(
            cfg.max_connections * max_instances + 5 <= 25,
            "default budget must fit the smallest Cloud SQL tier"
        );
    }

    /// `describe_target` is printed to the terminal before a mutation, so the
    /// one thing it must never do is leak the password.
    #[test]
    fn describe_target_never_prints_the_password() {
        // Built from fields, the password is simply never one of them.
        assert_eq!(describe_target(&config()), "h:5432/d");

        let mut cfg = config();
        cfg.url = Some(SecretString::from(
            "postgres://appuser:sup3rs3cret@db.example.com:5432/anime?sslmode=require",
        ));
        let from_url = describe_target(&cfg);
        assert!(
            !from_url.contains("sup3rs3cret"),
            "password leaked from url: {from_url}"
        );
        assert!(
            !from_url.contains("appuser"),
            "username leaked from url: {from_url}"
        );
        assert_eq!(from_url, "db.example.com:5432/anime?sslmode=require");
    }

    /// A DSN with no credentials must still describe its target rather than
    /// falling apart on the missing `@`.
    #[test]
    fn describe_target_handles_a_url_without_credentials() {
        let mut cfg = config();
        cfg.url = Some(SecretString::from("postgres://db.internal/anime"));
        assert_eq!(describe_target(&cfg), "db.internal/anime");
    }

    /// Cloud SQL's Unix-socket form puts the instance in a query parameter and
    /// has an empty host. It must survive redaction intact, because that string
    /// is the only way to tell staging from production at a glance.
    #[test]
    fn describe_target_keeps_the_cloud_sql_socket_path() {
        let mut cfg = config();
        cfg.url = Some(SecretString::from(
            "postgres://appuser:pw@/anime?host=/cloudsql/proj:europe-west1:anime-prod",
        ));
        let described = describe_target(&cfg);
        assert!(
            !described.contains("pw@"),
            "credentials leaked: {described}"
        );
        assert_eq!(
            described,
            "/anime?host=/cloudsql/proj:europe-west1:anime-prod"
        );
    }

    /// Managed providers hand out a full string with query parameters
    /// (`?sslmode=require`) that rebuilding from fields would silently drop.
    #[test]
    fn url_wins_and_is_passed_through_verbatim() {
        let mut cfg = config();
        cfg.url = Some(SecretString::from(
            "postgres://x:y@ep-a.neon.tech/neondb?sslmode=require",
        ));
        assert_eq!(
            connection_string(&cfg).expose_secret(),
            "postgres://x:y@ep-a.neon.tech/neondb?sslmode=require"
        );
    }
}
