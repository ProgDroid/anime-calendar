use std::{convert::Infallible, str::FromStr};

use crate::entity::calendar::Language;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, sqlx::FromRow)]
pub struct UserSettings {
    pub user_id: i32,
    pub theme_preference: Theme,
    pub language_preference: SiteLanguage,
    pub title_language_preference: Language,
    pub timezone: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "theme", rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl FromStr for Theme {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "light" => Self::Light,
            _ => Self::Dark,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "site_language", rename_all = "lowercase")]
pub enum SiteLanguage {
    #[default]
    En,
    Pt,
}

impl FromStr for SiteLanguage {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pt" => Self::Pt,
            _ => Self::En,
        })
    }
}
