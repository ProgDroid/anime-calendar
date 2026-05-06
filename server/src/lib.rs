pub mod cache;
pub mod config;
pub mod controllers;
pub mod entity;
pub mod error;
pub mod mappers;
pub mod metrics;
pub mod middleware;
pub mod openapi;
pub mod redis_pubsub;
pub mod server;
pub mod services;
#[cfg(test)]
pub mod test_helpers;

pub type ServerResult<T> = std::result::Result<T, error::Error>;
