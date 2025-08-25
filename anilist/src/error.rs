use thiserror::Error;

// TODO expand

#[derive(Error, Debug)]
pub enum AnilistError {
    #[error("Reqwest Error: `{0}`")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Anilist Error: `{0}`")]
    GenericError(String),
    #[error("Successful request with no errors has no response data")]
    MissingData,
}
