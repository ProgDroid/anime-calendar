use std::{convert::Infallible, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Native,
    Romaji,
}

impl FromStr for Language {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "english" => Self::English,
            "romaji" => Self::Romaji,
            _ => Self::Native,
        })
    }
}
