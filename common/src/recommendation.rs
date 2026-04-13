use serde::{Deserialize, Serialize};

use crate::{id::Id, media_cover::MediaCover, title::Title};

#[derive(Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RecommendationMedia {
    #[cfg_attr(feature = "utoipa", schema(value_type = i64))]
    pub id: Id,
    pub id_mal: Option<i64>,
    pub title: Title,
    pub cover_image: MediaCover,
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Recommendation {
    pub rating: i64,
    pub media: RecommendationMedia,
}
