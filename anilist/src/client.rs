use graphql_client::{GraphQLQuery, Response};
use log::{debug, error, info};
use reqwest::Client as ReqwestClient;
use std::time::Duration;

use crate::{
    Result, error::AnilistError, query::{
        get_items::{GetItems, get_items},
        search_items::{SearchItems, search_items},
        search_items_by_type::{SearchItemsByType, search_items_by_type},
    }
};

const API_URL: &str = "https://graphql.anilist.co/";

#[derive(Clone)]
pub struct Client {
    client: ReqwestClient,
}

impl Client {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: ReqwestClient::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build Anilist HTTP client"),
        }
    }

    /// # Errors
    /// Returns `ReqwestError` if request or response parsing fails
    /// Returns `GenericError` if response contains errors
    /// Returns `MissingData` if there's no errors or data in the response
    pub async fn get_item(&self, id: i64) -> Result<get_items::ResponseData> {
        info!("Getting item with id: {id}");

        let request_body = GetItems::build_query(get_items::Variables { ids: vec![id] });

        let res = self
            .client
            .post(API_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_body: Response<get_items::ResponseData> = res.json().await?;

        if let Some(errors) = response_body.errors
            && !errors.is_empty() {
                error!("{errors:?}");
                return Err(AnilistError::GenericError(format!("{errors:?}")));
            }

        response_body.data.map_or_else(
            || Err(AnilistError::MissingData),
            |data| {
                info!("{data:?}");
                Ok(data)
            },
        )
    }

    /// # Errors
    /// Returns `ReqwestError` if request or response parsing fails
    /// Returns `GenericError` if response contains errors
    /// Returns `MissingData` if there's no errors or data in the response
    pub async fn get_items(&self, ids: Vec<i64>) -> Result<get_items::ResponseData> {
        info!("Getting items with IDs: {ids:?}");

        let request_body = GetItems::build_query(get_items::Variables { ids });

        debug!("{:?}", request_body.query);

        let res = self
            .client
            .post(API_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_body: Response<get_items::ResponseData> = res.json().await?;

        if let Some(errors) = response_body.errors
            && !errors.is_empty() {
                error!("{errors:?}");
                return Err(AnilistError::GenericError(format!("{errors:?}")));
            }

        response_body.data.map_or_else(
            || Err(AnilistError::MissingData),
            |data| {
                info!("{data:?}");
                Ok(data)
            },
        )
    }

    /// # Errors
    /// Returns `ReqwestError` if request or response parsing fails
    /// Returns `GenericError` if response contains errors
    /// Returns `MissingData` if there's no errors or data in the response
    pub async fn search_items(&self, name: String) -> Result<search_items::ResponseData> {
        info!("Searching for {name}");

        let request_body = SearchItems::build_query(search_items::Variables { name });

        debug!("{:?}", request_body.query);

        let res = self
            .client
            .post(API_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_body: Response<search_items::ResponseData> = res.json().await?;

        if let Some(errors) = response_body.errors
            && !errors.is_empty() {
                error!("{errors:?}");
                return Err(AnilistError::GenericError(format!("{errors:?}")));
            }
        response_body.data.map_or_else(
            || Err(AnilistError::MissingData),
            |data| {
                info!("{data:?}");
                Ok(data)
            },
        )
    }

    /// # Errors
    /// Returns `ReqwestError` if request or response parsing fails
    /// Returns `GenericError` if response contains errors
    /// Returns `MissingData` if there's no errors or data in the response
    pub async fn search_items_by_type(&self, name: String, media_type: search_items_by_type::MediaType) -> Result<search_items_by_type::ResponseData> {
        info!("Searching for {name} with type {media_type:?}");

        let request_body = SearchItemsByType::build_query(search_items_by_type::Variables { name, media_type });

        debug!("{:?}", request_body.query);

        let res = self
            .client
            .post(API_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_body: Response<search_items_by_type::ResponseData> = res.json().await?;

        if let Some(errors) = response_body.errors
            && !errors.is_empty() {
                error!("{errors:?}");
                return Err(AnilistError::GenericError(format!("{errors:?}")));
            }
        response_body.data.map_or_else(
            || Err(AnilistError::MissingData),
            |data| {
                info!("{data:?}");
                Ok(data)
            },
        )
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
