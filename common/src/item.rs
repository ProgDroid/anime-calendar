use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    error::Error, id::Id, media_cover::MediaCover, recommendation::Recommendation,
    schedule::Schedule, title::Title,
};

#[derive(Serialize, Deserialize, Clone)]
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

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Anime => "ANIME",
                Self::Manga => "MANGA",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Item {
    pub id: Id,
    pub id_mal: Option<i64>,
    pub title: Title,
    pub airing_schedule: Vec<Schedule>,
    pub episode_duration: i64,
    pub media_type: Type,
    pub cover_image: MediaCover,
    pub banner_image: String,
    pub recommendations: Vec<Recommendation>,
}

pub trait Repository {
    fn get_item(&self, id: Id) -> impl std::future::Future<Output = Option<Item>> + Send;

    fn get_items(&self, ids: Vec<Id>) -> impl std::future::Future<Output = Vec<Item>> + Send;

    fn search_items(
        &self,
        name: String,
        media_type: Option<Type>,
    ) -> impl std::future::Future<Output = Vec<Item>> + Send;
}
