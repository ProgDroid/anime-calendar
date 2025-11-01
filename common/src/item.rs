use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{error::Error, id::Id, schedule::Schedule, title::Title};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Type {
    Anime,
    Manga,
}

impl FromStr for Type {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ANIME" => Ok(Self::Anime),
            "MANGA" => Ok(Self::Manga),
            _ => Err(Self::Err::UnknownMediaType),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub id: Id,
    pub id_mal: Option<i64>,
    pub title: Title,
    pub airing_schedule: Vec<Schedule>,
    pub episode_duration: i64,
    pub media_type: Type,
}

pub trait Repository {
    fn get_item(&self, id: u64) -> impl std::future::Future<Output = Option<Item>> + Send; // TODO should input here be Id type

    fn get_items(&self, ids: Vec<u64>) -> impl std::future::Future<Output = Vec<Item>> + Send; // TODO should input here be Id types
}
