use secrecy::ExposeSecret as _;
use server::{
    ServerResult,
    cache::Cache,
    config::{database::Database as DatabaseConfig, server::Server as ServerConfig},
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, email_verification::EmailVerificationMapper,
        google_oauth::GoogleOauth, password_reset::PasswordResetMapper,
        refresh_token::RefreshTokenMapper, stripe_event::StripeEventMapper,
        subscription::SubscriptionMapper, user::UserMapper, user_settings::UserSettingsMapper,
    },
    services::{email::EmailService, entitlement::EntitlementService},
};
use stripe::Client as StripeClient;

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
    let subscription_mapper = SubscriptionMapper::new(db_config.clone()).await?;
    let stripe_event_mapper = StripeEventMapper::new(db_config.clone()).await?;
    let email_service = EmailService::new(settings.smtp.clone());
    let entitlement_service = EntitlementService::new(subscription_mapper.clone());

    // Stripe client uses an empty secret when not configured — this keeps
    // dev environments where Stripe isn't set up runnable. Handlers that need
    // Stripe gate on `StripeConfig::is_configured()` and surface a 500 with
    // `Error::StripeNotConfigured` rather than panic at startup.
    let stripe_client = StripeClient::new(settings.stripe.secret_key.expose_secret().to_string());
    let stripe_config = settings.stripe.clone();

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
        subscription_mapper,
        stripe_event_mapper,
        email_service,
        entitlement_service,
        stripe_client,
        stripe_config,
    )?
    .await?)
}
// TODO add setting for adding specific episode times rather than all day settings
