pub mod cache;
pub mod config;
pub mod controllers;
pub mod entity;
pub mod error;
pub mod mappers;
pub mod middleware;
pub mod openapi;
pub mod server;
pub mod services;
pub mod utils;

use crate::{
    cache::Cache,
    config::{database::Database as DatabaseConfig, server::Server as ServerConfig},
    error::Error,
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, google_oauth::GoogleOauth,
        password_reset::PasswordResetMapper, user::UserMapper, user_settings::UserSettingsMapper,
    },
    services::email::EmailService,
};

pub type ServerResult<T> = std::result::Result<T, Error>;

#[actix_web::main]
async fn main() -> ServerResult<()> {
    let settings = ServerConfig::new().expect("Failed to load config");

    let anilist = Anilist::new();
    let db_config = DatabaseConfig::new()?;

    let user_mapper = UserMapper::new(db_config.clone()).await?;
    let user_settings_mapper = UserSettingsMapper::new(db_config.clone()).await?;

    let calendar_mapper = CalendarMapper::new(db_config.clone()).await?;

    let google_oauth = GoogleOauth::new(&settings.google_client_id);
    let token_mapper = PasswordResetMapper::new(db_config.clone()).await?;
    let email_service = EmailService::new(settings.smtp.clone());

    // Initialize Redis cache
    let cache = Cache::new(
        &settings.redis.host,
        settings.redis.port,
        &settings.redis.password,
        settings.redis.db,
    )
    .await
    .expect("Failed to initialize Redis cache");

    Ok(server::start(
        settings,
        anilist,
        google_oauth,
        cache,
        user_mapper,
        calendar_mapper,
        user_settings_mapper,
        token_mapper,
        email_service,
    )?
    .await?)
}
// TODO add setting for adding specific episode times rather than all day settings
