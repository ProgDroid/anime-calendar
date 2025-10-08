use anilist::{client::Client, ResponseItem};
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
    async fn get_item(&self, id: u64) -> Option<Item> {
        match self.client.get_item(id).await {
            Ok(item) => response_to_item(item),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }
}

fn response_to_item(response: ResponseItem) -> Option<Item> {
    match response.media {
        Some(media) => {
            let airing_schedule: Vec<Schedule> = match media.airing_schedule {
                Some(connection) => connection.nodes.map_or_else(Vec::default, |entries| {
                    entries
                        .iter()
                        .flatten()
                        .map(|item| Schedule {
                            id: Id::new(item.id).unwrap(),                      // TODO handle
                            airing_at: Timestamp::new(item.airing_at).unwrap(), // TODO handle
                            episode: item.episode,
                            media_id: None,
                        })
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

            let media_type = match media.media_type {
                Some(media_type) => match media_type {
                    anilist::MediaType::ANIME => Type::Anime,
                    anilist::MediaType::MANGA => Type::Manga,
                    _ => Type::Anime,
                },
                None => Type::Anime, // TODO probably should do this differently
            };

            Some(Item {
                id: Id::new(media.id).unwrap(), // TODO handle
                id_mal: media.id_mal,
                title,
                airing_schedule,
                episode_duration: media.duration.unwrap_or_default(),
                media_type,
            })
        }
        None => None,
    }
}
