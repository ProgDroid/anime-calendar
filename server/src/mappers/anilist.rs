use anilist::{client::Client, GetItemsResponseItem};
use common::{
    id::Id,
    item::{Item, Repository as RepositoryItem, Type},
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
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl RepositoryItem for Anilist {
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
}

fn response_to_items(response: GetItemsResponseItem) -> Vec<Item> {
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

            if let Some(id) = Id::new(media.id) {
                items.push(Item {
                    id,
                    id_mal: media.id_mal,
                    title,
                    airing_schedule,
                    episode_duration: media.duration.unwrap_or_default(),
                    media_type,
                });
            }
        }
    }

    items
}
