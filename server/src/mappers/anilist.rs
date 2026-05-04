use std::str::FromStr;

use anilist::{
    AiringScheduleTrait, CoverImageTrait, MediaTrait, RecommendationMediaTrait,
    RecommendationTrait, ResponseItemTrait, SearchItemsByTypeMediaType, TitleTrait, client::Client,
};
use common::{
    id::Id,
    item::{AnimeDataSource, Item, Type},
    media_cover::MediaCover,
    recommendation::{Recommendation, RecommendationMedia},
    schedule::Schedule,
    timestamp::Timestamp,
    title::Title,
};
use log::error;

#[derive(Clone)]
pub struct Anilist {
    client: Client,
}

impl Anilist {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for Anilist {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimeDataSource for Anilist {
    async fn get_item(&self, id: Id) -> Option<Item> {
        match std::convert::TryInto::<i64>::try_into(id.to_int()) {
            Ok(id) => match self.client.get_item(id).await {
                Ok(item) => response_to_items(item).first().cloned(),
                Err(e) => {
                    error!("{e}");
                    None
                }
            },
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }

    async fn get_items(&self, ids: Vec<Id>) -> Vec<Item> {
        let ids: Vec<i64> = ids
            .iter()
            .map(|id| id.to_int().try_into())
            .filter_map(std::result::Result::ok)
            .collect();

        match self.client.get_items(ids).await {
            Ok(item) => response_to_items(item),
            Err(e) => {
                error!("{e}");
                Vec::default()
            }
        }
    }

    async fn search_items(&self, query: String, media_type: Option<Type>) -> Vec<Item> {
        match media_type {
            Some(media_type) => {
                let inner_type = match media_type {
                    Type::Anime => SearchItemsByTypeMediaType::ANIME,
                    Type::Manga => SearchItemsByTypeMediaType::MANGA,
                };

                match self.client.search_items_by_type(query, inner_type).await {
                    Ok(item) => response_to_items(item),
                    Err(e) => {
                        error!("{e}");
                        Vec::default()
                    }
                }
            }
            None => match self.client.search_items(query).await {
                Ok(item) => response_to_items(item),
                Err(e) => {
                    error!("{e}");
                    Vec::default()
                }
            },
        }
    }
}

fn response_to_items<T: ResponseItemTrait>(response: T) -> Vec<Item> {
    response
        .media()
        .iter()
        .filter_map(|media| {
            let media_id = Id::new(media.id());

            media_id.as_ref()?;

            let media_type = Type::from_str(&media.media_type());

            if media_type.is_err() {
                return None;
            }

            let media_title = media.title();

            let title = Title {
                english: media_title.english(),
                native: media_title.native(),
                romaji: media_title.romaji(),
            };

            let media_airing_schedule = media.airing_schedule();

            let airing_schedule = media_airing_schedule
                .iter()
                .filter_map(|schedule| {
                    if let Some(id) = Id::new(schedule.id())
                        && let Some(airing_at) = Timestamp::new(schedule.airing_at())
                    {
                        Some(Schedule {
                            id,
                            airing_at,
                            episode: schedule.episode(),
                            media_id: None,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let media_cover_image = media.cover_image();

            let cover_image = MediaCover {
                extra_large: media_cover_image.extra_large(),
                large: media_cover_image.large(),
                medium: media_cover_image.medium(),
                color: media_cover_image.color(),
            };

            let recommendations = media
                .recommendations()
                .iter()
                .filter_map(|recommendation| {
                    let recommendation_media = recommendation.media();
                    let media_id = Id::new(recommendation_media.id());

                    media_id.as_ref()?;

                    let media_title = recommendation_media.title();

                    let title = Title {
                        english: media_title.english(),
                        native: media_title.native(),
                        romaji: media_title.romaji(),
                    };

                    let media_cover_image = recommendation_media.cover_image();

                    let cover_image = MediaCover {
                        extra_large: media_cover_image.extra_large(),
                        large: media_cover_image.large(),
                        medium: media_cover_image.medium(),
                        color: media_cover_image.color(),
                    };

                    Some(Recommendation {
                        rating: recommendation.rating(),
                        media: RecommendationMedia {
                            id: media_id.unwrap(),
                            id_mal: recommendation_media.id_mal(),
                            title,
                            cover_image,
                        },
                    })
                })
                .collect();

            Some(Item {
                id: media_id.unwrap(),
                id_mal: media.id_mal(),
                title,
                airing_schedule,
                episode_duration: media.episode_duration(),
                media_type: media_type.unwrap(),
                cover_image,
                banner_image: media.banner_image(),
                recommendations,
            })
        })
        .collect()
}
