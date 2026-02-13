use std::str::FromStr;

use actix_cors::Cors;
use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
use env_logger::Builder;
use log::{error, LevelFilter};

use crate::{
    cache::Cache,
    config::server::Server as ServerConfig,
    controllers::{auth, cache_metrics, calendar, item, items, oauth, user},
    mappers::{anilist::Anilist, database::Database, google_oauth::GoogleOauth},
    ServerResult,
};

#[derive(Clone)]
pub struct Repos {
    pub anilist: Anilist,
    pub database: Database,
    pub google_oauth: GoogleOauth,
    pub cache: Cache,
}

/// # Errors
/// Returns an error if the server fails to start.
pub fn start(
    config: &ServerConfig,
    anilist: Anilist,
    database: Database,
    google_oauth: GoogleOauth,
    cache: Cache,
) -> ServerResult<Server> {
    let repos = Repos {
        anilist,
        database,
        google_oauth,
        cache,
    };

    let level_filter = match LevelFilter::from_str(&config.log_level) {
        Ok(filter) => filter,
        Err(e) => {
            error!("Invalid log level: {e}");
            LevelFilter::Info
        }
    };

    Builder::default().filter_level(level_filter).init();

    Ok(HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .app_data(web::Data::new(repos.clone()))
            .service(item::get)
            .service(items::get)
            .service(items::search)
            .service(calendar::export)
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
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .run())
}
// TODO refactor frontend components
// TODO ensure cache keys are per user

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
