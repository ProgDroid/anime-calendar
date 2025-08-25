use std::{convert::Infallible, str::FromStr};

pub enum Language {
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
