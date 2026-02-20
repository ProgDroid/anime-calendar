pub mod client;
pub mod error;
mod query;

pub use query::get_items::get_items::GetItemsPageMedia;
pub use query::get_items::get_items::MediaType as GetItemsMediaType;
pub use query::get_items::get_items::ResponseData as GetItemsResponseItem;
pub use query::search_items::search_items::MediaType as SearchItemsMediaType;
pub use query::search_items::search_items::ResponseData as SearchItemsResponseItem;
pub use query::search_items::search_items::SearchItemsPageMedia;
pub use query::search_items_by_type::search_items_by_type::MediaType as SearchItemsByTypeMediaType;
pub use query::search_items_by_type::search_items_by_type::ResponseData as SearchItemsByTypeResponseItem;
pub use query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMedia;

use crate::error::AnilistError;
use std::result;

type Result<T> = result::Result<T, AnilistError>;

pub trait AiringScheduleTrait {
    fn id(&self) -> i64;

    fn airing_at(&self) -> i64;

    fn episode(&self) -> i64;
}

impl AiringScheduleTrait for query::get_items::get_items::GetItemsPageMediaAiringScheduleNodes {
    fn id(&self) -> i64 {
        self.id
    }

    fn airing_at(&self) -> i64 {
        self.airing_at
    }

    fn episode(&self) -> i64 {
        self.episode
    }
}

impl AiringScheduleTrait
    for query::search_items::search_items::SearchItemsPageMediaAiringScheduleNodes
{
    fn id(&self) -> i64 {
        self.id
    }

    fn airing_at(&self) -> i64 {
        self.airing_at
    }

    fn episode(&self) -> i64 {
        self.episode
    }
}

impl AiringScheduleTrait
    for query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaAiringScheduleNodes
{
    fn id(&self) -> i64 {
        self.id
    }

    fn airing_at(&self) -> i64 {
        self.airing_at
    }

    fn episode(&self) -> i64 {
        self.episode
    }
}

pub trait TitleTrait {
    fn english(&self) -> String;

    fn native(&self) -> String;

    fn romaji(&self) -> String;
}

impl TitleTrait for query::get_items::get_items::GetItemsPageMediaTitle {
    fn english(&self) -> String {
        self.english.clone().unwrap_or_default()
    }

    fn native(&self) -> String {
        self.native.clone().unwrap_or_default()
    }

    fn romaji(&self) -> String {
        self.romaji.clone().unwrap_or_default()
    }
}

impl TitleTrait for query::get_items::get_items::GetItemsPageMediaRecommendationsEdgesNodeMediaRecommendationTitle {
    fn english(&self) -> String {
        self.english.clone().unwrap_or_default()
    }

    fn native(&self) -> String {
        self.native.clone().unwrap_or_default()
    }

    fn romaji(&self) -> String {
        self.romaji.clone().unwrap_or_default()
    }
}

impl TitleTrait for query::search_items::search_items::SearchItemsPageMediaTitle {
    fn english(&self) -> String {
        self.english.clone().unwrap_or_default()
    }

    fn native(&self) -> String {
        self.native.clone().unwrap_or_default()
    }

    fn romaji(&self) -> String {
        self.romaji.clone().unwrap_or_default()
    }
}

impl TitleTrait for query::search_items::search_items::SearchItemsPageMediaRecommendationsEdgesNodeMediaRecommendationTitle {
    fn english(&self) -> String {
        self.english.clone().unwrap_or_default()
    }

    fn native(&self) -> String {
        self.native.clone().unwrap_or_default()
    }

    fn romaji(&self) -> String {
        self.romaji.clone().unwrap_or_default()
    }
}

impl TitleTrait
    for query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaTitle
{
    fn english(&self) -> String {
        self.english.clone().unwrap_or_default()
    }

    fn native(&self) -> String {
        self.native.clone().unwrap_or_default()
    }

    fn romaji(&self) -> String {
        self.romaji.clone().unwrap_or_default()
    }
}

impl TitleTrait for query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaRecommendationsEdgesNodeMediaRecommendationTitle {
    fn english(&self) -> String {
        self.english.clone().unwrap_or_default()
    }

    fn native(&self) -> String {
        self.native.clone().unwrap_or_default()
    }

    fn romaji(&self) -> String {
        self.romaji.clone().unwrap_or_default()
    }
}

pub trait CoverImageTrait {
    fn extra_large(&self) -> String;

    fn large(&self) -> String;

    fn medium(&self) -> String;

    fn color(&self) -> String;
}

impl CoverImageTrait for query::get_items::get_items::GetItemsPageMediaCoverImage {
    fn extra_large(&self) -> String {
        self.extra_large.clone().unwrap_or_default()
    }

    fn large(&self) -> String {
        self.large.clone().unwrap_or_default()
    }

    fn medium(&self) -> String {
        self.medium.clone().unwrap_or_default()
    }

    fn color(&self) -> String {
        self.color.clone().unwrap_or_default()
    }
}

impl CoverImageTrait for query::get_items::get_items::GetItemsPageMediaRecommendationsEdgesNodeMediaRecommendationCoverImage {
    fn extra_large(&self) -> String {
        self.extra_large.clone().unwrap_or_default()
    }

    fn large(&self) -> String {
        self.large.clone().unwrap_or_default()
    }

    fn medium(&self) -> String {
        self.medium.clone().unwrap_or_default()
    }

    fn color(&self) -> String {
        self.color.clone().unwrap_or_default()
    }
}

impl CoverImageTrait for query::search_items::search_items::SearchItemsPageMediaCoverImage {
    fn extra_large(&self) -> String {
        self.extra_large.clone().unwrap_or_default()
    }

    fn large(&self) -> String {
        self.large.clone().unwrap_or_default()
    }

    fn medium(&self) -> String {
        self.medium.clone().unwrap_or_default()
    }

    fn color(&self) -> String {
        self.color.clone().unwrap_or_default()
    }
}

impl CoverImageTrait for query::search_items::search_items::SearchItemsPageMediaRecommendationsEdgesNodeMediaRecommendationCoverImage {
    fn extra_large(&self) -> String {
        self.extra_large.clone().unwrap_or_default()
    }

    fn large(&self) -> String {
        self.large.clone().unwrap_or_default()
    }

    fn medium(&self) -> String {
        self.medium.clone().unwrap_or_default()
    }

    fn color(&self) -> String {
        self.color.clone().unwrap_or_default()
    }
}

impl CoverImageTrait
    for query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaCoverImage
{
    fn extra_large(&self) -> String {
        self.extra_large.clone().unwrap_or_default()
    }

    fn large(&self) -> String {
        self.large.clone().unwrap_or_default()
    }

    fn medium(&self) -> String {
        self.medium.clone().unwrap_or_default()
    }

    fn color(&self) -> String {
        self.color.clone().unwrap_or_default()
    }
}

impl CoverImageTrait for query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaRecommendationsEdgesNodeMediaRecommendationCoverImage {
    fn extra_large(&self) -> String {
        self.extra_large.clone().unwrap_or_default()
    }

    fn large(&self) -> String {
        self.large.clone().unwrap_or_default()
    }

    fn medium(&self) -> String {
        self.medium.clone().unwrap_or_default()
    }

    fn color(&self) -> String {
        self.color.clone().unwrap_or_default()
    }
}

pub trait RecommendationMediaTrait {
    fn id(&self) -> i64;

    fn id_mal(&self) -> Option<i64>;

    fn title(&self) -> impl TitleTrait;

    fn cover_image(&self) -> impl CoverImageTrait;
}

impl RecommendationMediaTrait
    for query::get_items::get_items::GetItemsPageMediaRecommendationsEdgesNodeMediaRecommendation
{
    fn id(&self) -> i64 {
        self.id
    }

    fn id_mal(&self) -> Option<i64> {
        self.id_mal
    }

    fn title(&self) -> impl TitleTrait {
        self.title.clone().unwrap_or(query::get_items::get_items::GetItemsPageMediaRecommendationsEdgesNodeMediaRecommendationTitle { romaji: None, english: None, native: None })
    }

    fn cover_image(&self) -> impl CoverImageTrait {
        self.cover_image.clone().unwrap_or(query::get_items::get_items::GetItemsPageMediaRecommendationsEdgesNodeMediaRecommendationCoverImage { extra_large: None, large: None, medium: None, color: None })
    }
}

impl RecommendationMediaTrait
    for query::search_items::search_items::SearchItemsPageMediaRecommendationsEdgesNodeMediaRecommendation
{
    fn id(&self) -> i64 {
        self.id
    }

    fn id_mal(&self) -> Option<i64> {
        self.id_mal
    }

    fn title(&self) -> impl TitleTrait {
        self.title.clone().unwrap_or(query::search_items::search_items::SearchItemsPageMediaRecommendationsEdgesNodeMediaRecommendationTitle { romaji: None, english: None, native: None })
    }

    fn cover_image(&self) -> impl CoverImageTrait {
        self.cover_image.clone().unwrap_or(query::search_items::search_items::SearchItemsPageMediaRecommendationsEdgesNodeMediaRecommendationCoverImage { extra_large: None, large: None, medium: None, color: None })
    }
}

impl RecommendationMediaTrait
    for query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaRecommendationsEdgesNodeMediaRecommendation
{
    fn id(&self) -> i64 {
        self.id
    }

    fn id_mal(&self) -> Option<i64> {
        self.id_mal
    }

    fn title(&self) -> impl TitleTrait {
        self.title.clone().unwrap_or(query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaRecommendationsEdgesNodeMediaRecommendationTitle { romaji: None, english: None, native: None })
    }

    fn cover_image(&self) -> impl CoverImageTrait {
        self.cover_image.clone().unwrap_or(query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaRecommendationsEdgesNodeMediaRecommendationCoverImage { extra_large: None, large: None, medium: None, color: None })
    }
}

pub trait RecommendationTrait {
    fn rating(&self) -> i64;

    fn media(&self) -> impl RecommendationMediaTrait;
}

impl RecommendationTrait
    for query::get_items::get_items::GetItemsPageMediaRecommendationsEdgesNode
{
    fn rating(&self) -> i64 {
        self.rating.unwrap_or_default()
    }

    fn media(&self) -> impl RecommendationMediaTrait {
        self.media_recommendation.clone().unwrap_or(query::get_items::get_items::GetItemsPageMediaRecommendationsEdgesNodeMediaRecommendation {
            id: 0,
            id_mal: None,
            title: None,
            cover_image: None
        })
    }
}

impl RecommendationTrait
    for query::search_items::search_items::SearchItemsPageMediaRecommendationsEdgesNode
{
    fn rating(&self) -> i64 {
        self.rating.unwrap_or_default()
    }

    fn media(&self) -> impl RecommendationMediaTrait {
        self.media_recommendation.clone().unwrap_or(query::search_items::search_items::SearchItemsPageMediaRecommendationsEdgesNodeMediaRecommendation {
            id: 0,
            id_mal: None,
            title: None,
            cover_image: None
        })
    }
}

impl RecommendationTrait
    for query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaRecommendationsEdgesNode
{
    fn rating(&self) -> i64 {
        self.rating.unwrap_or_default()
    }

    fn media(&self) -> impl RecommendationMediaTrait {
        self.media_recommendation.clone().unwrap_or(query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaRecommendationsEdgesNodeMediaRecommendation {
            id: 0,
            id_mal: None,
            title: None,
            cover_image: None
        })
    }
}

pub trait MediaTrait {
    fn id(&self) -> i64;

    fn id_mal(&self) -> Option<i64>;

    fn airing_schedule(&self) -> Vec<impl AiringScheduleTrait>;

    fn episode_duration(&self) -> i64;

    fn title(&self) -> impl TitleTrait;

    fn media_type(&self) -> String;

    fn cover_image(&self) -> impl CoverImageTrait;

    fn banner_image(&self) -> String;

    fn recommendations(&self) -> Vec<impl RecommendationTrait>;
}

impl MediaTrait for GetItemsPageMedia {
    fn id(&self) -> i64 {
        self.id
    }

    fn id_mal(&self) -> Option<i64> {
        self.id_mal
    }

    fn airing_schedule(&self) -> Vec<impl AiringScheduleTrait> {
        self.airing_schedule
            .clone()
            .map_or_else(Vec::default, |schedule| {
                schedule
                    .nodes
                    .map_or_else(Vec::default, |item| item.into_iter().flatten().collect())
            })
    }

    fn episode_duration(&self) -> i64 {
        self.duration.unwrap_or_default()
    }

    fn title(&self) -> impl TitleTrait {
        self.title
            .clone()
            .unwrap_or(query::get_items::get_items::GetItemsPageMediaTitle {
                english: None,
                native: None,
                romaji: None,
            })
    }

    fn media_type(&self) -> String {
        String::from(
            self.media_type
                .clone()
                .map_or("ANIME", |media_type| match media_type {
                    GetItemsMediaType::ANIME => "ANIME",
                    _ => "MANGA",
                }),
        )
    }

    fn cover_image(&self) -> impl CoverImageTrait {
        self.cover_image.clone().unwrap_or(
            query::get_items::get_items::GetItemsPageMediaCoverImage {
                extra_large: None,
                large: None,
                medium: None,
                color: None,
            },
        )
    }

    fn banner_image(&self) -> String {
        self.banner_image.clone().unwrap_or_default()
    }

    fn recommendations(&self) -> Vec<impl RecommendationTrait> {
        self.recommendations
            .clone()
            .map_or_else(Vec::default, |recommendations| {
                recommendations
                    .edges
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .map(|edges| edges.node)
                    .collect()
            })
    }
}

impl MediaTrait for SearchItemsPageMedia {
    fn id(&self) -> i64 {
        self.id
    }

    fn id_mal(&self) -> Option<i64> {
        self.id_mal
    }

    fn airing_schedule(&self) -> Vec<impl AiringScheduleTrait> {
        self.airing_schedule
            .clone()
            .map_or_else(Vec::default, |schedule| {
                schedule
                    .nodes
                    .map_or_else(Vec::default, |item| item.into_iter().flatten().collect())
            })
    }

    fn episode_duration(&self) -> i64 {
        self.duration.unwrap_or_default()
    }

    fn title(&self) -> impl TitleTrait {
        self.title.clone().unwrap_or(
            query::search_items::search_items::SearchItemsPageMediaTitle {
                english: None,
                native: None,
                romaji: None,
            },
        )
    }

    fn media_type(&self) -> String {
        String::from(
            self.media_type
                .clone()
                .map_or("ANIME", |media_type| match media_type {
                    SearchItemsMediaType::ANIME => "ANIME",
                    _ => "MANGA",
                }),
        )
    }

    fn cover_image(&self) -> impl CoverImageTrait {
        self.cover_image.clone().unwrap_or(
            query::search_items::search_items::SearchItemsPageMediaCoverImage {
                extra_large: None,
                large: None,
                medium: None,
                color: None,
            },
        )
    }

    fn banner_image(&self) -> String {
        self.banner_image.clone().unwrap_or_default()
    }

    fn recommendations(&self) -> Vec<impl RecommendationTrait> {
        self.recommendations
            .clone()
            .map_or_else(Vec::default, |recommendations| {
                recommendations
                    .edges
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .map(|edges| edges.node)
                    .collect()
            })
    }
}

impl MediaTrait for SearchItemsByTypePageMedia {
    fn id(&self) -> i64 {
        self.id
    }

    fn id_mal(&self) -> Option<i64> {
        self.id_mal
    }

    fn airing_schedule(&self) -> Vec<impl AiringScheduleTrait> {
        self.airing_schedule
            .clone()
            .map_or_else(Vec::default, |schedule| {
                schedule
                    .nodes
                    .map_or_else(Vec::default, |item| item.into_iter().flatten().collect())
            })
    }

    fn episode_duration(&self) -> i64 {
        self.duration.unwrap_or_default()
    }

    fn title(&self) -> impl TitleTrait {
        self.title.clone().unwrap_or(
            query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaTitle {
                english: None,
                native: None,
                romaji: None,
            },
        )
    }

    fn media_type(&self) -> String {
        String::from(
            self.media_type
                .clone()
                .map_or("ANIME", |media_type| match media_type {
                    SearchItemsByTypeMediaType::ANIME => "ANIME",
                    _ => "MANGA",
                }),
        )
    }

    fn cover_image(&self) -> impl CoverImageTrait {
        self.cover_image.clone().unwrap_or(
            query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMediaCoverImage {
                extra_large: None,
                large: None,
                medium: None,
                color: None,
            },
        )
    }

    fn banner_image(&self) -> String {
        self.banner_image.clone().unwrap_or_default()
    }

    fn recommendations(&self) -> Vec<impl RecommendationTrait> {
        self.recommendations
            .clone()
            .map_or_else(Vec::default, |recommendations| {
                recommendations
                    .edges
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .map(|edges| edges.node)
                    .collect()
            })
    }
}

pub trait ResponseItemTrait {
    fn media(self) -> Vec<impl MediaTrait>;
}

impl ResponseItemTrait for GetItemsResponseItem {
    fn media(self) -> Vec<impl MediaTrait> {
        self.page.media.unwrap_or_default()
    }
}

impl ResponseItemTrait for SearchItemsResponseItem {
    fn media(self) -> Vec<impl MediaTrait> {
        self.page.media.unwrap_or_default()
    }
}

impl ResponseItemTrait for SearchItemsByTypeResponseItem {
    fn media(self) -> Vec<impl MediaTrait> {
        self.page.media.unwrap_or_default()
    }
}
