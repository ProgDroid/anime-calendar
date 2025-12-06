use config::{Config, ConfigError, File};
use secrecy::SecretString;
use serde::Deserialize;

const DATABASE_FILE: &str = "database.toml";

#[allow(clippy::struct_field_names)]
#[derive(Deserialize, Clone)]
pub struct Database {
    pub user: String,
    pub pass: SecretString,
    pub host: String,
    pub port: String,
    pub database: String,
}

impl Database {
    pub fn new() -> std::result::Result<Self, ConfigError> {
        let database_config = Config::builder()
            .add_source(File::with_name(DATABASE_FILE))
            .build()?;

        database_config.try_deserialize()
    }
}
