use anilist::{client::Client, GetItemsResponseItem, SearchItemsResponseItem, SearchItemsByTypeResponseItem, SearchItemsByTypeMediaType};
use common::{
    id::Id, item::{Item, Repository as RepositoryItem, Type}, media_cover::MediaCover, schedule::Schedule, timestamp::Timestamp, title::Title
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

impl RepositoryItem for Anilist {
    async fn get_item(&self, id: Id) -> Option<Item> {
        match std::convert::TryInto::<i64>::try_into(id.to_int()) {
            Ok(id) => match self.client.get_item(id).await {
                Ok(item) => response_to_items_get(item).first().cloned(),
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
            Ok(item) => response_to_items_get(item),
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
                    Ok(item) => response_to_items_search_by_type(item),
                    Err(e) => {
                        error!("{e}");
                        Vec::default()
                    }
                }
            }
            None => match self.client.search_items(query).await {
                Ok(item) => response_to_items_search(item),
                Err(e) => {
                    error!("{e}");
                    Vec::default()
                }
            }
        }
    }

}

// TODO this is bad, fix somehow
// impl ResponseItem that gets everything?
fn response_to_items_get(response: GetItemsResponseItem) -> Vec<Item> {
    let mut items = Vec::default();

    if let Some(media_list) = response.page.media {
        for media in media_list {
            let airing_schedule: Vec<Schedule> = match media.airing_schedule {
                Some(connection) => connection.nodes.map_or_else(Vec::default, |entries| {
                    entries
                        .iter()
                        .flatten()
                        .filter_map(|item| {
                            if let Some(id) = Id::new(item.id) && let Some(airing_at) = Timestamp::new(item.airing_at) {
                                Some(Schedule {
                                    id,
                                    airing_at,
                                    episode: item.episode,
                                    media_id: None,
                                })
                            } else {
                                None
                            }}
                        )
                        .collect()
                }),
                None => Vec::default(),
            };

            let title = match media.title {
                Some(title) => Title {
                    english: title.english.unwrap_or_default(),
                    native: title.native.unwrap_or_default(),
                    romaji: title.romaji.unwrap_or_default(),
                },
                None => Title {
                    english: String::default(),
                    native: String::default(),
                    romaji: String::default(),
                },
            };

            let media_type = media
                .media_type
                .map_or(Type::Anime, |media_type| match media_type {
                    anilist::GetItemsMediaType::MANGA => Type::Manga,
                    _ => Type::Anime,
                });

            let cover_image = match media.cover_image {
                Some(image) => MediaCover {
                    extra_large: image.extra_large.unwrap_or_default(),
                    large: image.large.unwrap_or_default(),
                    medium: image.medium.unwrap_or_default(),
                    color: image.color.unwrap_or_default(),
                },
                None => MediaCover {
                    extra_large: String::default(),
                    large: String::default(),
                    medium: String::default(),
                    color: String::default(),
                }
            };

            if let Some(id) = Id::new(media.id) {
                items.push(Item {
                    id,
                    id_mal: media.id_mal,
                    title,
                    airing_schedule,
                    episode_duration: media.duration.unwrap_or_default(),
                    media_type,
                    cover_image,
                    banner_image: media.banner_image.unwrap_or_default(),
                });
            }
        }
    }

    items
}

fn response_to_items_search(response: SearchItemsResponseItem) -> Vec<Item> {
    let mut items = Vec::default();

    if let Some(media_list) = response.page.media {
        for media in media_list {
            let airing_schedule: Vec<Schedule> = match media.airing_schedule {
                Some(connection) => connection.nodes.map_or_else(Vec::default, |entries| {
                    entries
                        .iter()
                        .flatten()
                        .filter_map(|item| {
                            if let Some(id) = Id::new(item.id) && let Some(airing_at) = Timestamp::new(item.airing_at) {
                                Some(Schedule {
                                    id,
                                    airing_at,
                                    episode: item.episode,
                                    media_id: None,
                                })
                            } else {
                                None
                            }}
                        )
                        .collect()
                }),
                None => Vec::default(),
            };

            let title = match media.title {
                Some(title) => Title {
                    english: title.english.unwrap_or_default(),
                    native: title.native.unwrap_or_default(),
                    romaji: title.romaji.unwrap_or_default(),
                },
                None => Title {
                    english: String::default(),
                    native: String::default(),
                    romaji: String::default(),
                },
            };

            let media_type = media
                .media_type
                .map_or(Type::Anime, |media_type| match media_type {
                    anilist::SearchItemsMediaType::MANGA => Type::Manga,
                    _ => Type::Anime,
                });

            let cover_image = match media.cover_image {
                Some(image) => MediaCover {
                    extra_large: image.extra_large.unwrap_or_default(),
                    large: image.large.unwrap_or_default(),
                    medium: image.medium.unwrap_or_default(),
                    color: image.color.unwrap_or_default(),
                },
                None => MediaCover {
                    extra_large: String::default(),
                    large: String::default(),
                    medium: String::default(),
                    color: String::default(),
                }
            };

            if let Some(id) = Id::new(media.id) {
                items.push(Item {
                    id,
                    id_mal: media.id_mal,
                    title,
                    airing_schedule,
                    episode_duration: media.duration.unwrap_or_default(),
                    media_type,
                    cover_image,
                    banner_image: media.banner_image.unwrap_or_default(),
                });
            }
        }
    }

    items
}

fn response_to_items_search_by_type(response: SearchItemsByTypeResponseItem) -> Vec<Item> {
    let mut items = Vec::default();

    if let Some(media_list) = response.page.media {
        for media in media_list {
            let airing_schedule: Vec<Schedule> = match media.airing_schedule {
                Some(connection) => connection.nodes.map_or_else(Vec::default, |entries| {
                    entries
                        .iter()
                        .flatten()
                        .filter_map(|item| {
                            if let Some(id) = Id::new(item.id) && let Some(airing_at) = Timestamp::new(item.airing_at) {
                                Some(Schedule {
                                    id,
                                    airing_at,
                                    episode: item.episode,
                                    media_id: None,
                                })
                            } else {
                                None
                            }}
                        )
                        .collect()
                }),
                None => Vec::default(),
            };

            let title = match media.title {
                Some(title) => Title {
                    english: title.english.unwrap_or_default(),
                    native: title.native.unwrap_or_default(),
                    romaji: title.romaji.unwrap_or_default(),
                },
                None => Title {
                    english: String::default(),
                    native: String::default(),
                    romaji: String::default(),
                },
            };

            let media_type = media
                .media_type
                .map_or(Type::Anime, |media_type| match media_type {
                    anilist::SearchItemsByTypeMediaType::MANGA => Type::Manga,
                    _ => Type::Anime,
                });

            let cover_image = match media.cover_image {
                Some(image) => MediaCover {
                    extra_large: image.extra_large.unwrap_or_default(),
                    large: image.large.unwrap_or_default(),
                    medium: image.medium.unwrap_or_default(),
                    color: image.color.unwrap_or_default(),
                },
                None => MediaCover {
                    extra_large: String::default(),
                    large: String::default(),
                    medium: String::default(),
                    color: String::default(),
                }
            };

            if let Some(id) = Id::new(media.id) {
                items.push(Item {
                    id,
                    id_mal: media.id_mal,
                    title,
                    airing_schedule,
                    episode_duration: media.duration.unwrap_or_default(),
                    media_type,
                    cover_image,
                    banner_image: media.banner_image.unwrap_or_default(),
                });
            }
        }
    }

    items
}
// TODO add export buttons for calendars (grid and view page)