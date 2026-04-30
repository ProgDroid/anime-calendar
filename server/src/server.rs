use std::str::FromStr;

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    dev::Server,
    middleware::{Compress, Condition, DefaultHeaders, Logger},
    web, App, HttpServer,
};
use env_logger::Builder;
use log::{error, LevelFilter};
use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    cache::Cache,
    config::server::{AppBaseUrl, CookieSettings, JwtSecret, Server as ServerConfig},
    controllers::{
        auth, calendar, email_verification, item, items, oauth, password_reset, refresh, user,
    },
    error::Error,
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, email_verification::EmailVerificationMapper,
        google_oauth::GoogleOauth, password_reset::PasswordResetMapper,
        refresh_token::RefreshTokenMapper, user::UserMapper, user_settings::UserSettingsMapper,
    },
    openapi::ApiDoc,
    services::email::EmailService,
    ServerResult,
};

#[allow(clippy::too_many_arguments)]
/// # Errors
/// Returns an error if the server fails to start.
pub fn start(
    config: ServerConfig,
    anilist: Anilist,
    google_oauth: GoogleOauth,
    cache: Cache,
    user_mapper: UserMapper,
    calendar_mapper: CalendarMapper,
    user_settings_mapper: UserSettingsMapper,
    token_mapper: PasswordResetMapper,
    verification_mapper: EmailVerificationMapper,
    refresh_token_mapper: RefreshTokenMapper,
    email_service: EmailService,
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

    let governor_conf = GovernorConfigBuilder::default()
        .seconds_per_request(1)
        .burst_size(60)
        .finish()
        .ok_or(Error::GovernorConfig)?;

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
            .configure(move |cfg| {
                if enable_docs {
                    cfg.service(
                        SwaggerUi::new("/swagger-ui/{_:.*}")
                            .url("/api-docs/openapi.json", ApiDoc::openapi()),
                    );
                }
            })
            .wrap(Condition::new(compress, Compress::default()))
            .wrap(crate::metrics::http::HttpMetrics)
            .wrap(
                Logger::new("%a \"%m %{safe_url}xi %H\" %s %b %T").custom_request_replace(
                    "safe_url",
                    |req| {
                        let path = req.path();
                        if path.starts_with("/calendars/subscribe/") {
                            "/calendars/subscribe/[redacted]".to_owned()
                        } else {
                            path.to_owned()
                        }
                    },
                ),
            )
            .wrap(Governor::new(&governor_conf))
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
            .app_data(web::Data::new(anilist.clone()))
            .app_data(web::Data::new(google_oauth.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(jwt_secret.clone()))
            .app_data(web::Data::new(cookie_settings.clone()))
            .app_data(web::Data::new(token_mapper.clone()))
            .app_data(web::Data::new(verification_mapper.clone()))
            .app_data(web::Data::new(refresh_token_mapper.clone()))
            .app_data(web::Data::new(email_service.clone()))
            .app_data(web::Data::new(app_base_url.clone()))
            .service(item::get)
            .service(items::get)
            .service(items::search)
            .service(calendar::export)
            .service(calendar::subscribe_feed)
            .service(calendar::put)
            .service(calendar::get_calendar)
            .service(calendar::get_calendars)
            .service(calendar::delete_calendar)
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
    })
    .bind(format!("{host}:{port}"))?
    .run())
}
