use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnilistError {
    #[error("Reqwest Error: `{0}`")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Anilist Error: `{0}`")]
    GenericError(String),
    #[error("Successful request with no errors has no response data")]
    MissingData,
    /// `AniList` replied 429. `retry_after_seconds` carries the `Retry-After`
    /// header when present so callers can schedule their own backoff.
    #[error("Anilist rate limited (retry after {retry_after_seconds:?} s)")]
    RateLimited { retry_after_seconds: Option<u64> },
    /// Any other non-success HTTP status (5xx, 4xx). Previously these fell
    /// through to JSON deserialization and surfaced as opaque reqwest errors.
    #[error("Anilist returned HTTP status {0}")]
    HttpStatus(u16),
}
