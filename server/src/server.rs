use std::str::FromStr;

use actix_cors::Cors;
use actix_web::{
    dev::Server,
    middleware::{Compress, Condition, Logger},
    web, App, HttpServer,
};
use env_logger::Builder;
use log::{error, LevelFilter};

use crate::{
    cache::Cache,
    config::server::Server as ServerConfig,
    controllers::{auth, cache_metrics, calendar, item, items, oauth, user},
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, google_oauth::GoogleOauth, user::UserMapper,
        user_settings::UserSettingsMapper,
    },
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

    Ok(HttpServer::new(move || {
        let config = config.clone();

        App::new()
            .wrap(Condition::new(compress, Compress::default()))
            .wrap(Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(), // TODO ?
            )
            .app_data(web::Data::new(user_mapper.clone()))
            .app_data(web::Data::new(calendar_mapper.clone()))
            .app_data(web::Data::new(user_settings_mapper.clone()))
            .app_data(web::Data::new(anilist.clone()))
            .app_data(web::Data::new(google_oauth.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(config)) // TODO do I want this data lying around the entire time?
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
// TODO refactor frontend components

// TODO add more tests for the backend services and controllers
// TODO implement rate limiting for API endpoints
// TODO optimize database queries and indexes
// TODO add logging to all critical sections of the code
// TODO improve error handling and provide meaningful error messages to the client
// TODO refactor the codebase to follow a more modular architecture
// TODO update dependencies to their latest versions
// TODO add support for multiple languages in the frontend
// TODO optimize image loading and rendering on the frontend
// TODO improve accessibility features on the frontend
// TODO add analytics tracking to the frontend
// TODO cache anilist content as well? separate cache
// TODO load testing
