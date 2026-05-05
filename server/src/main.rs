use secrecy::ExposeSecret as _;
use server::{
    ServerResult,
    cache::Cache,
    config::{database::Database as DatabaseConfig, server::Server as ServerConfig},
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, calendar_editor::CalendarEditorMapper,
        email_verification::EmailVerificationMapper, google_oauth::GoogleOauth,
        password_reset::PasswordResetMapper, refresh_token::RefreshTokenMapper,
        stripe_event::StripeEventMapper, subscription::SubscriptionMapper, user::UserMapper,
        user_settings::UserSettingsMapper,
    },
    services::{
        cached_anilist::CachedAnilist, email::EmailService, entitlement::EntitlementService,
        sharing_authz::SharingAuthz,
    },
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
    // Dedicated pool for the show-count service. Cheap (Arc-backed) and
    // keeps EntitlementService independent of any single mapper's lifetime.
    let show_count_pool = server::mappers::database::Database::new(db_config.clone())
        .await?
        .pool;
    let show_count_service =
        server::services::show_count::ShowCountService::new(show_count_pool);
    let entitlement_service = EntitlementService::new(
        subscription_mapper.clone(),
        show_count_service.clone(),
        &settings.limits,
    );

    let calendar_editor_mapper = CalendarEditorMapper::new(db_config.clone()).await?;
    let sharing_authz =
        SharingAuthz::new(calendar_editor_mapper.clone(), entitlement_service.clone());

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

    // Reconcile loop: hourly safety net for the Stripe webhook path. Skips
    // itself if Stripe isn't configured (no point hitting Stripe with an
    // empty secret). `interval_secs = 0` also disables it, used in tests.
    if settings.stripe.is_configured() {
        let fetcher = server::services::reconcile::LiveStripeFetcher::new(stripe_client.clone());
        server::services::reconcile::spawn_loop(
            subscription_mapper.clone(),
            fetcher,
            settings.reconcile.interval_secs,
        );
    } else {
        log::info!("reconcile: stripe not configured, loop not spawned");
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

    let cached_anilist = CachedAnilist::new(anilist, cache.clone(), &settings.cache);

    // Dedicated pool for ICS export + frozen blob services. Cheap (Arc-backed)
    // and keeps these services independent of any single mapper's lifetime.
    let ics_pool = server::mappers::database::Database::new(db_config.clone())
        .await?
        .pool;
    let ics_export = server::services::ics_export::IcsExportService::new(
        ics_pool.clone(),
        cached_anilist.clone(),
        user_settings_mapper.clone(),
        entitlement_service.clone(),
    );
    let frozen_ics =
        server::services::frozen_ics::FrozenIcsService::new(ics_pool, ics_export.clone());

    // Dedicated pool for the PUT /calendar advisory-locked transaction.
    // Cheap (Arc-backed) and keeps the controller independent of any single
    // mapper's lifetime.
    let controller_pool = server::mappers::database::Database::new(db_config.clone())
        .await?
        .pool;

    Ok(server::server::start(
        settings,
        cached_anilist,
        ics_export,
        frozen_ics,
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
        show_count_service,
        stripe_client,
        stripe_config,
        controller_pool,
        sharing_authz,
    )?
    .await?)
}
// TODO add setting for adding specific episode times rather than all day settings
