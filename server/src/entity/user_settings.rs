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
    pub date_display_preference: DateFormat,
    pub date_separator_preference: DateSeparator,
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

#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "date_format")]
pub enum DateFormat {
    #[default]
    #[sqlx(rename = "YYYY-MM-DD")]
    YyyyMmDd,
    #[sqlx(rename = "DD-MM-YYYY")]
    DdMmYyyy,
    #[sqlx(rename = "MM-DD-YYYY")]
    MmDdYyyy,
}

impl FromStr for DateFormat {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "DD-MM-YYYY" => Self::DdMmYyyy,
            "MM-DD-YYYY" => Self::MmDdYyyy,
            _ => Self::YyyyMmDd,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "date_separator")]
pub enum DateSeparator {
    #[sqlx(rename = "/")]
    Slash,
    #[default]
    #[sqlx(rename = "-")]
    Dash,
}

impl FromStr for DateSeparator {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "/" => Self::Slash,
            _ => Self::Dash,
        })
    }
}
