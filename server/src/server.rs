use std::str::FromStr;

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    dev::Server,
    middleware::{Compress, Condition, Logger},
    web, App, HttpServer,
};
use env_logger::Builder;
use log::{error, LevelFilter};
use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    cache::Cache,
    config::server::{CookieSettings, JwtSecret, Server as ServerConfig},
    controllers::{auth, cache_metrics, calendar, item, items, oauth, user},
    error::Error,
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, google_oauth::GoogleOauth, user::UserMapper,
        user_settings::UserSettingsMapper,
    },
    openapi::ApiDoc,
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
    let allowed_origins = config.allowed_origins.clone();
    let jwt_secret = JwtSecret::new(config.jwt_secret);
    let cookie_settings = CookieSettings {
        secure: config.cookie_secure,
    };

    let governor_conf = GovernorConfigBuilder::default()
        .seconds_per_request(1)
        .burst_size(60)
        .finish()
        .ok_or(Error::GovernorConfig)?;

    Ok(HttpServer::new(move || {
        let cors = if allowed_origins.is_empty() {
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
        } else {
            let mut cors = Cors::default();
            for origin in &allowed_origins {
                cors = cors.allowed_origin(origin);
            }
            cors.allow_any_method()
                .allow_any_header()
                .supports_credentials()
        };

        App::new()
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .wrap(Condition::new(compress, Compress::default()))
            .wrap(Logger::default())
            .wrap(Governor::new(&governor_conf))
            .wrap(cors)
            .app_data(web::Data::new(user_mapper.clone()))
            .app_data(web::Data::new(calendar_mapper.clone()))
            .app_data(web::Data::new(user_settings_mapper.clone()))
            .app_data(web::Data::new(anilist.clone()))
            .app_data(web::Data::new(google_oauth.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(jwt_secret.clone()))
            .app_data(web::Data::new(cookie_settings.clone()))
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
            .service(user::get_user_details)
            .service(user::update_user)
            .service(user::delete_user)
            .service(user::update_password)
            .service(oauth::google_oauth)
            .service(cache_metrics::get_cache_metrics)
            .service(cache_metrics::get_cache_performance)
            .service(cache_metrics::get_cache_health)
            .service(cache_metrics::reset_metrics)
            .service(cache_metrics::get_cache_stats)
            .service(cache_metrics::flush_cache)
            .service(user::get_user_settings)
            .service(user::update_user_settings)
    })
    .bind(format!("{host}:{port}"))?
    .run())
}
