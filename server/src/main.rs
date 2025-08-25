mod config;
mod controllers;
mod mappers;
mod server;
mod services;

use std::io::Result;

use crate::{config::Server as ServerConfig, mappers::anilist::Anilist};

#[actix_web::main]
async fn main() -> Result<()> {
    let anilist = Anilist::new();
    // let database = Postgres::new();

    let settings = ServerConfig::new().expect("Failed to load config");

    server::start(&settings, anilist)?.await
}
// TODO add setting for adding specific episode times rather than all day settings
