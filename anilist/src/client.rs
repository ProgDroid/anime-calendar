use graphql_client::{GraphQLQuery, Response};
use log::{debug, error, info, warn};
use reqwest::{Client as ReqwestClient, StatusCode};
use std::time::Duration;

use crate::{
    Result,
    error::AnilistError,
    query::{
        get_items::{GetItems, get_items},
        search_items::{SearchItems, search_items},
        search_items_by_type::{SearchItemsByType, search_items_by_type},
    },
};

const API_URL: &str = "https://graphql.anilist.co/";

/// Must match the `perPage` literal in `queries/get_items.graphql`. `AniList`
/// caps page size at 50; id batches larger than this are chunked into
/// multiple requests so no item is silently dropped.
const MAX_IDS_PER_REQUEST: usize = 50;

/// A 429 with `Retry-After` at or below this is retried once in-place;
/// anything longer is surfaced to the caller instead of blocking a user
/// request on a sleep.
const MAX_INLINE_RETRY_SECS: u64 = 5;

#[derive(Clone)]
pub struct Client {
    client: ReqwestClient,
}

impl Client {
    /// # Panics
    /// It can panic if the duration here is changed to an invalid one
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: ReqwestClient::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build Anilist HTTP client"),
        }
    }

    /// Send one GraphQL query and decode the response.
    ///
    /// Single point for HTTP-status handling: a 429 honouring a short
    /// `Retry-After` is retried once; longer throttles and other non-success
    /// statuses become typed errors instead of falling through to JSON
    /// deserialization (which used to surface them as opaque reqwest errors
    /// while callers kept hammering the throttled upstream).
    ///
    /// # Errors
    /// Returns `RateLimited` on 429, `HttpStatus` on other non-success
    /// statuses, `ReqwestError` if the request or body decode fails,
    /// `GenericError` if the response contains GraphQL errors, and
    /// `MissingData` when a successful response carries no data.
    async fn execute<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData>
    where
        Q: GraphQLQuery,
        Q::ResponseData: std::fmt::Debug,
    {
        let request_body = Q::build_query(variables);
        debug!("{:?}", request_body.query);

        let mut retried = false;
        loop {
            let res = self
                .client
                .post(API_URL)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .json(&request_body)
                .send()
                .await?;

            let status = res.status();
            if status == StatusCode::TOO_MANY_REQUESTS {
                let retry_after_seconds = res
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.trim().parse::<u64>().ok());
                if !retried
                    && let Some(secs) = retry_after_seconds
                    && secs <= MAX_INLINE_RETRY_SECS
                {
                    warn!("anilist: rate limited, retrying once after {secs}s");
                    tokio::time::sleep(Duration::from_secs(secs)).await;
                    retried = true;
                    continue;
                }
                warn!("anilist: rate limited (Retry-After: {retry_after_seconds:?})");
                return Err(AnilistError::RateLimited {
                    retry_after_seconds,
                });
            }
            if !status.is_success() {
                error!("anilist: HTTP {status}");
                return Err(AnilistError::HttpStatus(status.as_u16()));
            }

            let response_body: Response<Q::ResponseData> = res.json().await?;

            if let Some(errors) = response_body.errors
                && !errors.is_empty()
            {
                error!("{errors:?}");
                return Err(AnilistError::GenericError(format!("{errors:?}")));
            }

            return response_body.data.map_or_else(
                || Err(AnilistError::MissingData),
                |data| {
                    debug!("{data:?}");
                    Ok(data)
                },
            );
        }
    }

    /// # Errors
    /// See [`Client::execute`].
    pub async fn get_item(&self, id: i64) -> Result<get_items::ResponseData> {
        info!("Getting item with id: {id}");
        self.execute::<GetItems>(get_items::Variables { ids: vec![id] })
            .await
    }

    /// Fetch media for a batch of ids, transparently chunking into
    /// `MAX_IDS_PER_REQUEST`-sized requests. `AniList`'s `Page` returns at most
    /// 50 entries (default 25), so a single oversized request would silently
    /// drop every id past the page boundary.
    ///
    /// # Errors
    /// See [`Client::execute`]. The first failing chunk aborts the batch.
    pub async fn get_items(&self, ids: Vec<i64>) -> Result<get_items::ResponseData> {
        info!("Getting {} items", ids.len());

        let mut merged: Vec<get_items::GetItemsPageMedia> = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(MAX_IDS_PER_REQUEST) {
            let data = self
                .execute::<GetItems>(get_items::Variables {
                    ids: chunk.to_vec(),
                })
                .await?;
            if let Some(media) = data.page.media {
                merged.extend(media);
            }
        }

        Ok(get_items::ResponseData {
            page: get_items::GetItemsPage {
                media: Some(merged),
            },
        })
    }

    /// # Errors
    /// See [`Client::execute`].
    pub async fn search_items(&self, name: String) -> Result<search_items::ResponseData> {
        info!("Searching for {name}");
        self.execute::<SearchItems>(search_items::Variables { name })
            .await
    }

    /// # Errors
    /// See [`Client::execute`].
    pub async fn search_items_by_type(
        &self,
        name: String,
        media_type: search_items_by_type::MediaType,
    ) -> Result<search_items_by_type::ResponseData> {
        info!("Searching for {name} with type {media_type:?}");
        self.execute::<SearchItemsByType>(search_items_by_type::Variables { name, media_type })
            .await
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
