use std::str::FromStr;

use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
use env_logger::Builder;
use log::LevelFilter;

use crate::{
    config::server::Server as ServerConfig,
    controllers::{calendar, item},
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

    Builder::default()
        .filter_level(LevelFilter::from_str(&config.log_level).unwrap()) // TODO fix
        .init();

    Ok(HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(repos.clone()))
            .service(item::get)
            .service(calendar::export)
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .run())
}
