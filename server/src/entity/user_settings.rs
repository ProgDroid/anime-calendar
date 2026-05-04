use std::{convert::Infallible, str::FromStr};

use crate::entity::calendar::Language;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, sqlx::FromRow, utoipa::ToSchema)]
#[serde(default)]
pub struct UserSettings {
    pub user_id: i32,
    pub theme_preference: Theme,
    pub language_preference: SiteLanguage,
    pub title_language_preference: Language,
    pub accent_preference: Accent,
    pub timezone: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// Per-user reminder offsets (minutes before episode air time). Free
    /// users have a single 30-minute heads-up; Pro users can store up to
    /// `LimitsConfig::pro_max_reminders` entries.
    #[serde(default = "default_reminder_offsets")]
    pub reminder_offsets_minutes: Vec<i32>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            user_id: 0,
            theme_preference: Theme::default(),
            language_preference: SiteLanguage::default(),
            title_language_preference: Language::default(),
            accent_preference: Accent::default(),
            timezone: String::new(),
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
            reminder_offsets_minutes: default_reminder_offsets(),
        }
    }
}

fn default_reminder_offsets() -> Vec<i32> {
    vec![30]
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy, utoipa::ToSchema)]
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

#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy, utoipa::ToSchema)]
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

#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "accent", rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Coral,
    Iris,
    Matcha,
    Sakura,
    Citron,
}

impl Accent {
    /// Pro accents are gated behind a paid tier. Must stay in lockstep with
    /// `frontend/src/constants/proAccents.ts::PRO_ACCENTS`.
    #[must_use]
    pub const fn is_pro(self) -> bool {
        matches!(self, Self::Matcha | Self::Sakura | Self::Citron)
    }
}

impl FromStr for Accent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "iris" => Self::Iris,
            "matcha" => Self::Matcha,
            "sakura" => Self::Sakura,
            "citron" => Self::Citron,
            _ => Self::Coral,
        })
    }
}
