use graphql_client::{GraphQLQuery, Response};
use log::{debug, error, info};
use reqwest::Client as ReqwestClient;

use crate::{
    error::AnilistError,
    query::{
        get_items::{get_items, GetItems},
    },
    Result,
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
            client: ReqwestClient::new(),
        }
    }

    /// # Errors
    /// Returns `ReqwestError` if request or response parsing fails
    /// Returns `GenericError` if response contains errors
    /// Returns `MissingData` if there's no errors or data in the response
    pub async fn get_item(&self, id: i64) -> Result<get_items::ResponseData> {
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
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
