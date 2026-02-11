use std::str::FromStr;

use actix_cors::Cors;
use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
use env_logger::Builder;
use log::{error, LevelFilter};

use crate::{
    config::server::Server as ServerConfig,
    controllers::{auth, calendar, item, items, oauth, user},
    mappers::{anilist::Anilist, database::Database, google_oauth::GoogleOauth},
    ServerResult,
};

#[derive(Clone)]
pub struct Repos {
    pub anilist: Anilist,
    pub database: Database,
    pub google_oauth: GoogleOauth,
}

/// # Errors
/// Returns an error if the server fails to start.
pub fn start(
    config: &ServerConfig,
    anilist: Anilist,
    database: Database,
    google_oauth: GoogleOauth,
) -> ServerResult<Server> {
    let repos = Repos {
        anilist,
        database,
        google_oauth,
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
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .run())
}
// TODO refactor frontend components
