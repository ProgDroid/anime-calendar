use std::str::FromStr;

use actix_cors::Cors;
use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
use env_logger::Builder;
use log::{error, LevelFilter};

use crate::{
    config::server::Server as ServerConfig,
    controllers::{calendar, item, items},
    mappers::{anilist::Anilist, database::Database},
    ServerResult,
};

#[derive(Clone)]
pub struct Repos {
    pub anilist: Anilist,
    pub database: Database,
}

pub fn start(config: &ServerConfig, anilist: Anilist, database: Database) -> ServerResult<Server> {
    let repos = Repos { anilist, database };

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
            .service(calendar::get)
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .run())
}
