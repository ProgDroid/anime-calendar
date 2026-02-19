use secrecy::ExposeSecret;
use sqlx::{PgPool, Pool};

use crate::{config::database::Database as Config, ServerResult};

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    /// # Errors
    /// Returns an error if the connection to the database fails
    pub async fn new(config: Config) -> ServerResult<Self> {
        let pool = Pool::connect(
            format!(
                "postgres://{}:{}@{}/{}",
                config.user,
                config.pass.expose_secret(),
                format_args!("{}:{}", config.host, config.port),
                config.database
            )
            .as_str(),
        )
        .await?;

        // Initialize with no cache initially - will be set by server
        Ok(Self { pool })
    }
}
