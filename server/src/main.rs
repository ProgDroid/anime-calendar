use server::{
    ServerResult,
    cache::Cache,
    config::{database::Database as DatabaseConfig, server::Server as ServerConfig},
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, email_verification::EmailVerificationMapper,
        google_oauth::GoogleOauth, password_reset::PasswordResetMapper,
        refresh_token::RefreshTokenMapper, user::UserMapper, user_settings::UserSettingsMapper,
    },
    services::email::EmailService,
};

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
    let verification_mapper = EmailVerificationMapper::new(db_config.clone()).await?;
    let refresh_token_mapper = RefreshTokenMapper::new(db_config.clone()).await?;
    let email_service = EmailService::new(settings.smtp.clone());

    if let Err(e) = server::metrics::init(&settings.metrics) {
        log::error!("failed to initialise metrics: {e}");
        // Don't abort startup — degraded mode without metrics is preferable
        // to a crashloop.
    }

    // Initialize Redis cache
    let cache = Cache::new(
        &settings.redis.host,
        settings.redis.port,
        &settings.redis.password,
        settings.redis.db,
    )
    .await
    .expect("Failed to initialize Redis cache");

    Ok(server::server::start(
        settings,
        anilist,
        google_oauth,
        cache,
        user_mapper,
        calendar_mapper,
        user_settings_mapper,
        token_mapper,
        verification_mapper,
        refresh_token_mapper,
        email_service,
    )?
    .await?)
}
// TODO add setting for adding specific episode times rather than all day settings
