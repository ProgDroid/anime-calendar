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
        cached_anilist::CachedAnilist, calendar_events::CalendarEventPublisher,
        email::EmailService, entitlement::EntitlementService, sharing_authz::SharingAuthz,
    },
};
use stripe::Client as StripeClient;

#[actix_web::main]
async fn main() -> ServerResult<()> {
    let settings = ServerConfig::new().expect("Failed to load config");
    if let Err(e) = settings.validate() {
        log::error!("{e}");
        return Err(server::error::Error::Server(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            e,
        )));
    }

    let anilist = Anilist::new();
    let db_config = DatabaseConfig::new()?;

    // ONE pool for the whole process. Every mapper and service below takes a
    // clone, which is Arc-backed and shares the same underlying pool.
    //
    // This used to be fifteen separate `Database::new` calls. At sqlx's default
    // of 10 connections per pool that is a ceiling of 150 per instance, and
    // Cloud Run multiplies it again by `max-instances` — against a db-f1-micro
    // limit of about 25. The budget is now a single number, `max_connections`
    // in database.toml; see its doc comment for the arithmetic.
    let db = server::mappers::database::Database::new(db_config).await?;

    let user_mapper = UserMapper::new(db.clone());
    let user_settings_mapper = UserSettingsMapper::new(db.clone());

    let calendar_mapper = CalendarMapper::new(db.clone());

    let google_oauth = GoogleOauth::new(&settings.google_client_id);
    let token_mapper = PasswordResetMapper::new(db.clone());
    let verification_mapper = EmailVerificationMapper::new(db.clone());
    let refresh_token_mapper = RefreshTokenMapper::new(db.clone());
    let subscription_mapper = SubscriptionMapper::new(db.clone());
    let stripe_event_mapper = StripeEventMapper::new(db.clone());
    let email_service = EmailService::new(settings.smtp.clone());
    let show_count_service = server::services::show_count::ShowCountService::new(db.pool.clone());
    let entitlement_service = EntitlementService::new(
        subscription_mapper.clone(),
        show_count_service.clone(),
        &settings.limits,
    );

    let calendar_editor_mapper = CalendarEditorMapper::new(db.clone());
    let calendar_invitation_mapper =
        server::mappers::calendar_invitation::CalendarInvitationMapper::new(db.clone());
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
    // Reconcile loop: spawned further below, after redis_pubsub and
    // calendar_event_publisher are initialised (sharing deps need them).

    // One resolved URL for every Redis consumer below. `redis.url` (a managed
    // provider's `rediss://` string) wins when set, otherwise it is assembled
    // from host/port/password. Resolving once is deliberate: the cache,
    // Pub/Sub and presence each dial Redis separately, and honouring the URL
    // in only some of them would leave the rest pointing at localhost.
    let redis_url = settings.redis.connection_url();

    // Initialize Redis cache
    let cache = Cache::from_url(redis_url.expose_secret())
        .await
        .expect("Failed to initialize Redis cache");

    // Initialize Redis Pub/Sub (publisher + per-channel subscriber tasks).
    // The URL is held as a SecretString so the embedded password is zeroed on
    // drop and stays out of Debug output.
    let redis_pubsub = server::redis_pubsub::RedisPubSub::from_url(redis_url.expose_secret())
        .await
        .expect("Failed to initialize Redis Pub/Sub");

    let calendar_event_publisher = CalendarEventPublisher::new(redis_pubsub.clone());

    // Reconcile loop: hourly safety net for the Stripe webhook path. Skips
    // itself if Stripe isn't configured (no point hitting Stripe with an
    // empty secret). `interval_secs = 0` also disables it, used in tests.
    if settings.stripe.is_configured() {
        let fetcher = server::services::reconcile::LiveStripeFetcher::new(stripe_client.clone());
        // A clone of the shared pool for the reconcile sharing-suspend
        // transaction. This one genuinely IS Arc-backed, unlike the
        // `Database::new` call it replaces.
        let reconcile_pool = db.pool.clone();
        let sharing_deps = server::services::reconcile::SharingDeps {
            editor_mapper: calendar_editor_mapper.clone(),
            invitation_mapper: calendar_invitation_mapper.clone(),
            publisher: calendar_event_publisher.clone(),
            email_service: email_service.clone(),
            user_mapper: user_mapper.clone(),
            pool: reconcile_pool,
        };
        server::services::reconcile::spawn_loop(
            subscription_mapper.clone(),
            fetcher,
            settings.reconcile.interval_secs,
            Some(sharing_deps),
        );
    } else {
        log::info!("reconcile: stripe not configured, loop not spawned");
    }

    let presence_service = server::services::presence::PresenceService::from_url(
        redis_url.expose_secret(),
        u64::from(settings.sharing.presence_ttl_seconds),
    )
    .await
    .expect("Failed to initialize PresenceService");

    let cached_anilist = CachedAnilist::new(anilist, cache.clone(), &settings.cache);

    // Shared pool for ICS export + frozen blob services.
    let ics_pool = db.pool.clone();
    let ics_export = server::services::ics_export::IcsExportService::new(
        ics_pool.clone(),
        cached_anilist.clone(),
        user_settings_mapper.clone(),
        entitlement_service.clone(),
    );
    let frozen_ics =
        server::services::frozen_ics::FrozenIcsService::new(ics_pool, ics_export.clone());

    // Backs the advisory-locked transactions in both PUT /calendar and
    // POST /calendars/{id}/items (add_item).
    //
    // Sharing one pool is safe here *because* these take `pg_advisory_xact_lock`:
    // the lock is held for the transaction, not the connection, so two callers
    // drawing from the same pool serialise exactly as they did when each had
    // its own. What does change is that the pool's `max_connections` is now the
    // ceiling for all of them together — if advisory-locked work ever starves,
    // raise that number rather than reintroducing a separate pool.
    let controller_pool = db.pool.clone();

    // Same pool for the InvitationService advisory-locked transactions.
    let invitation_pool = db.pool.clone();
    let invitation_service = server::services::invitation_service::InvitationService::new(
        calendar_invitation_mapper.clone(),
        email_service.clone(),
        invitation_pool,
        settings.sharing.clone(),
        settings.app_base_url.clone(),
    );

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
        calendar_editor_mapper,
        calendar_invitation_mapper,
        invitation_service,
        redis_pubsub,
        calendar_event_publisher,
        presence_service,
    )?
    .await?)
}
// TODO add setting for adding specific episode times rather than all day settings
