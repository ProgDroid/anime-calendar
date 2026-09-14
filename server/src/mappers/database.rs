use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, Pool};

use crate::{ServerResult, config::database::Database as Config};

/// The DSN to connect with: `config.url` when set (Neon and Cloud SQL hand out
/// a full connection string, often with required query parameters such as
/// `sslmode=require` that a field-by-field rebuild would drop), otherwise one
/// assembled from the individual fields.
///
/// Returned as a `SecretString` because it embeds the password.
fn connection_string(config: &Config) -> SecretString {
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

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    /// # Errors
    /// Returns an error if the connection to the database fails
    pub async fn new(config: Config) -> ServerResult<Self> {
        let pool = Pool::connect(connection_string(&config).expose_secret()).await?;

        // Initialize with no cache initially - will be set by server
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
        }
    }

    #[test]
    fn builds_dsn_from_individual_fields_when_url_is_unset() {
        assert_eq!(
            connection_string(&config()).expose_secret(),
            "postgres://u:p@h:5432/d"
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
