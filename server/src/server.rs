use std::str::FromStr;
use std::time::Duration;

use actix_cors::Cors;
use actix_web::{
    App, HttpServer,
    dev::Server,
    middleware::{Compress, Condition, DefaultHeaders, Logger},
    web,
};
use env_logger::Builder;
use log::{LevelFilter, error};
#[cfg(feature = "swagger-ui")]
use utoipa::OpenApi as _;
#[cfg(feature = "swagger-ui")]
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    ServerResult,
    cache::Cache,
    config::server::{AppBaseUrl, CookieSettings, JwtSecret, Server as ServerConfig, StripeConfig},
    controllers::{
        account, auth, calendar, email_verification, health, item, items, oauth, password_reset,
        public_config, refresh, stripe as stripe_controller,
        stripe_webhook as stripe_webhook_controller, subscription as subscription_controller, user,
    },
    error::Error,
    mappers::{
        calendar::CalendarMapper, email_verification::EmailVerificationMapper,
        google_oauth::GoogleOauth, password_reset::PasswordResetMapper,
        refresh_token::RefreshTokenMapper, stripe_event::StripeEventMapper,
        subscription::SubscriptionMapper, user::UserMapper, user_settings::UserSettingsMapper,
    },
    services::{
        cached_anilist::CachedAnilist, calendar_events::CalendarEventPublisher,
        email::EmailService, entitlement::EntitlementService, frozen_ics::FrozenIcsService,
        ics_export::IcsExportService, show_count::ShowCountService,
    },
};
use stripe::Client as StripeClient;

#[cfg(feature = "swagger-ui")]
use crate::openapi::ApiDoc;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
/// # Errors
/// Returns an error if the server fails to start.
pub fn start(
    config: ServerConfig,
    cached_anilist: CachedAnilist,
    ics_export: IcsExportService,
    frozen_ics: FrozenIcsService,
    google_oauth: GoogleOauth,
    cache: Cache,
    user_mapper: UserMapper,
    calendar_mapper: CalendarMapper,
    user_settings_mapper: UserSettingsMapper,
    token_mapper: PasswordResetMapper,
    verification_mapper: EmailVerificationMapper,
    refresh_token_mapper: RefreshTokenMapper,
    subscription_mapper: SubscriptionMapper,
    stripe_event_mapper: StripeEventMapper,
    email_service: EmailService,
    entitlement_service: EntitlementService,
    show_count_service: ShowCountService,
    stripe_client: StripeClient,
    stripe_config: StripeConfig,
    pg_pool: sqlx::PgPool,
    sharing_authz: crate::services::sharing_authz::SharingAuthz,
    calendar_editor_mapper: crate::mappers::calendar_editor::CalendarEditorMapper,
    calendar_invitation_mapper: crate::mappers::calendar_invitation::CalendarInvitationMapper,
    invitation_service: crate::services::invitation_service::InvitationService,
    redis_pubsub: crate::redis_pubsub::RedisPubSub,
    calendar_event_publisher: CalendarEventPublisher,
    presence_service: crate::services::presence::PresenceService,
) -> ServerResult<Server> {
    let level_filter = match LevelFilter::from_str(&config.log_level) {
        Ok(filter) => filter,
        Err(e) => {
            error!("Invalid log level: {e}");
            LevelFilter::Info
        }
    };

    Builder::default().filter_level(level_filter).init();

    let host = config.host.clone();
    let port = config.port;
    let compress = config.compress;
    let enable_docs = config.enable_docs;
    #[cfg(not(feature = "swagger-ui"))]
    if enable_docs {
        log::warn!(
            "enable_docs = true but this binary was built without the `swagger-ui` feature — /swagger-ui/ will not be served"
        );
    }
    let allowed_origins = config.allowed_origins.clone();
    if allowed_origins.is_empty() {
        return Err(Error::Config(config::ConfigError::Message(
            "allowed_origins must not be empty — refusing to start with open CORS".into(),
        )));
    }
    let jwt_secret = JwtSecret::new(config.jwt_secret);
    if jwt_secret.expose_secret().is_empty() {
        return Err(Error::Config(config::ConfigError::Message(
            "jwt_secret must not be empty — refusing to start with an empty signing key".into(),
        )));
    }
    let cookie_settings = CookieSettings {
        secure: config.cookie_secure,
    };
    let app_base_url = AppBaseUrl::new(config.app_base_url);
    let sharing_config = config.sharing.clone();

    // Snapshot the subset of config we expose to the SPA at bootstrap. Built
    // once here so the controller can't reach into other ServerConfig fields.
    let public_config = public_config::PublicConfig {
        google_client_id: config.google_client_id,
        limits: public_config::PublicLimits {
            free_calendar_limit: config.limits.free_calendar_limit,
            free_show_cap: config.limits.free_show_cap,
            pro_max_reminders: config.limits.pro_max_reminders,
        },
        presence_heartbeat_seconds: config.sharing.presence_heartbeat_seconds,
    };

    let rate_limit = crate::middleware::rate_limit::RateLimit::new(
        60,
        Duration::from_secs(1),
        config.trust_proxy_header,
    );

    let sse_connection_tracker = crate::services::sse_connection_tracker::SseConnectionTracker::new(
        sharing_config.sse_max_connections_per_user as usize,
    );

    Ok(HttpServer::new(move || {
        let cors = {
            let mut cors = Cors::default();
            for origin in &allowed_origins {
                cors = cors.allowed_origin(origin);
            }
            cors.allow_any_method()
                .allow_any_header()
                .supports_credentials()
        };

        App::new()
            .configure(move |_cfg| {
                #[cfg(feature = "swagger-ui")]
                if enable_docs {
                    _cfg.service(
                        SwaggerUi::new("/swagger-ui/{_:.*}")
                            .url("/api-docs/openapi.json", ApiDoc::openapi()),
                    );
                }
            })
            .wrap(Condition::new(compress, Compress::default()))
            .wrap(crate::metrics::http::HttpMetrics)
            .wrap(
                Logger::new("%a \"%m %{safe_url}xi %H\" %s %b %T")
                    .custom_request_replace("safe_url", |req| redact_path(req.path())),
            )
            .wrap(rate_limit.clone())
            .wrap(cors)
            .wrap(
                DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Content-Security-Policy", "default-src 'none'")),
            )
            .app_data(web::JsonConfig::default().limit(1_048_576))
            .app_data(web::Data::new(user_mapper.clone()))
            .app_data(web::Data::new(calendar_mapper.clone()))
            .app_data(web::Data::new(user_settings_mapper.clone()))
            .app_data(web::Data::new(cached_anilist.clone()))
            .app_data(web::Data::new(ics_export.clone()))
            .app_data(web::Data::new(frozen_ics.clone()))
            .app_data(web::Data::new(google_oauth.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(jwt_secret.clone()))
            .app_data(web::Data::new(cookie_settings.clone()))
            .app_data(web::Data::new(token_mapper.clone()))
            .app_data(web::Data::new(verification_mapper.clone()))
            .app_data(web::Data::new(refresh_token_mapper.clone()))
            .app_data(web::Data::new(subscription_mapper.clone()))
            .app_data(web::Data::new(stripe_event_mapper.clone()))
            .app_data(web::Data::new(email_service.clone()))
            .app_data(web::Data::new(entitlement_service.clone()))
            .app_data(web::Data::new(show_count_service.clone()))
            .app_data(web::Data::new(stripe_client.clone()))
            .app_data(web::Data::new(stripe_config.clone()))
            .app_data(web::Data::new(app_base_url.clone()))
            .app_data(web::Data::new(public_config.clone()))
            .app_data(web::Data::new(pg_pool.clone()))
            .app_data(web::Data::new(sharing_authz.clone()))
            .app_data(web::Data::new(calendar_editor_mapper.clone()))
            .app_data(web::Data::new(calendar_invitation_mapper.clone()))
            .app_data(web::Data::new(invitation_service.clone()))
            .app_data(web::Data::new(sharing_config.clone()))
            .app_data(web::Data::new(redis_pubsub.clone()))
            .app_data(web::Data::new(calendar_event_publisher.clone()))
            .app_data(web::Data::new(presence_service.clone()))
            .app_data(web::Data::new(sse_connection_tracker.clone()))
            .service(public_config::get)
            .service(health::health)
            .service(item::get)
            .service(items::get)
            .service(items::search)
            .service(calendar::export)
            .service(calendar::subscribe_feed)
            .service(calendar::put)
            .service(calendar::get_calendar)
            .service(calendar::get_calendars)
            .service(calendar::delete_calendar)
            .service(calendar::add_item)
            .service(calendar::remove_item)
            .service(auth::login)
            .service(auth::register)
            .service(auth::get_current_user)
            .service(auth::verify_token_endpoint)
            .service(auth::logout)
            .service(refresh::refresh)
            .service(password_reset::forgot_password)
            .service(password_reset::reset_password)
            .service(email_verification::verify_email)
            .service(email_verification::resend_verification)
            .service(user::get_user_details)
            .service(user::update_user)
            .service(user::delete_user)
            .service(user::update_password)
            .service(oauth::google_oauth)
            .service(user::get_user_settings)
            .service(user::update_user_settings)
            .service(stripe_controller::create_checkout_session)
            .service(stripe_controller::create_portal_session)
            .service(stripe_webhook_controller::stripe_webhook)
            .service(subscription_controller::get_my_subscription)
            .service(account::get_usage)
            .service(crate::controllers::sharing::create_invitation)
            .service(crate::controllers::sharing::revoke_invitation)
            .service(crate::controllers::sharing::resend_invitation)
            .service(crate::controllers::sharing::list_members)
            .service(crate::controllers::sharing::remove_editor)
            .service(crate::controllers::sharing::leave_calendar)
            .service(crate::controllers::sharing::preview_invitation)
            .service(crate::controllers::sharing::accept_invitation)
            .service(crate::controllers::sharing::decline_invitation)
            .service(crate::controllers::sharing::presence_heartbeat)
            .service(
                web::resource("/calendars/{id}/events")
                    .route(web::get().to(crate::controllers::sse::calendar_events)),
            )
    })
    .bind(format!("{host}:{port}"))?
    .run())
}

/// Scrub high-entropy URL-segment secrets from a request path before it
/// reaches access logs. Both the iCal subscribe URL and the co-editor
/// invitation token grant access on possession alone (no auth header), so
/// leaking the raw path into stdout is equivalent to leaking the secret.
///
/// Matches the prefix and replaces the next path segment with `[redacted]`;
/// anything after the secret segment (e.g. `/accept`, `/decline` for
/// invitations) is preserved so log readers can still distinguish the
/// action.
fn redact_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("/calendars/subscribe/") {
        let tail = rest.find('/').map_or("", |i| &rest[i..]);
        format!("/calendars/subscribe/[redacted]{tail}")
    } else if let Some(rest) = path.strip_prefix("/invitations/") {
        let tail = rest.find('/').map_or("", |i| &rest[i..]);
        format!("/invitations/[redacted]{tail}")
    } else {
        path.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::redact_path;

    #[test]
    fn passes_through_paths_without_secrets() {
        assert_eq!(redact_path("/api/v1/users"), "/api/v1/users");
        assert_eq!(redact_path("/calendars/42"), "/calendars/42");
        assert_eq!(
            redact_path("/invite/some-frontend-route"),
            "/invite/some-frontend-route"
        );
        assert_eq!(redact_path("/"), "/");
    }

    #[test]
    fn redacts_subscribe_token() {
        assert_eq!(
            redact_path("/calendars/subscribe/aBc123-very-secret-token"),
            "/calendars/subscribe/[redacted]",
        );
    }

    #[test]
    fn redacts_invitation_token_preview() {
        assert_eq!(
            redact_path("/invitations/64-byte-hex-token-deadbeef"),
            "/invitations/[redacted]",
        );
    }

    #[test]
    fn preserves_action_segment_after_invitation_token() {
        assert_eq!(
            redact_path("/invitations/64-byte-hex-token-deadbeef/accept"),
            "/invitations/[redacted]/accept",
        );
        assert_eq!(
            redact_path("/invitations/64-byte-hex-token-deadbeef/decline"),
            "/invitations/[redacted]/decline",
        );
    }

    #[test]
    fn does_not_match_partial_prefix() {
        // `/invitationsX` must not be treated as `/invitations/`.
        assert_eq!(redact_path("/invitationsfoo"), "/invitationsfoo");
        assert_eq!(
            redact_path("/calendars/subscribefoo"),
            "/calendars/subscribefoo"
        );
    }
}
