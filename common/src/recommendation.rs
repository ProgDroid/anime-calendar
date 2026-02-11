use serde::{Deserialize, Serialize};

use crate::{id::Id, media_cover::MediaCover, title::Title};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RecommendationMedia {
    pub id: Id,
    pub id_mal: Option<i64>,
    pub title: Title,
    pub cover_image: MediaCover,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Recommendation {
    pub rating: i64,
    pub media: RecommendationMedia,
}
