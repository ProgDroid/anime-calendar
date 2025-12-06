mod config;
mod controllers;
mod entity;
mod error;
mod mappers;
mod server;
mod services;

use crate::{
    config::{database::Database as DatabaseConfig, server::Server as ServerConfig},
    error::Error,
    mappers::{anilist::Anilist, database::Database},
};

pub type ServerResult<T> = std::result::Result<T, Error>;

#[actix_web::main]
async fn main() -> ServerResult<()> {
    let anilist = Anilist::new();
    let database = Database::new(DatabaseConfig::new()?).await?;

    let settings = ServerConfig::new().expect("Failed to load config");

    Ok(server::start(&settings, anilist, database)?.await?)
}
// TODO add setting for adding specific episode times rather than all day settings
