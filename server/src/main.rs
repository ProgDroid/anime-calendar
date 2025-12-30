pub mod config;
pub mod controllers;
pub mod entity;
pub mod error;
pub mod mappers;
pub mod middleware;
pub mod server;
pub mod services;

use crate::{
    config::{database::Database as DatabaseConfig, server::Server as ServerConfig},
    error::Error,
    mappers::anilist::Anilist,
};
use mappers::database::Database;

pub type ServerResult<T> = std::result::Result<T, Error>;

#[actix_web::main]
async fn main() -> ServerResult<()> {
    let anilist = Anilist::new();
    let database = Database::new(DatabaseConfig::new()?).await?;

    let settings = ServerConfig::new().expect("Failed to load config");

    Ok(server::start(&settings, anilist, database)?.await?)
}
// TODO add setting for adding specific episode times rather than all day settings
