use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, sqlx::Type, Copy, utoipa::ToSchema)]
#[sqlx(type_name = "language", rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Native,
    Romaji,
}

impl Language {
    #[must_use]
    pub const fn to_common_language(&self) -> common::language::Language {
        match self {
            Self::English => common::language::Language::English,
            Self::Native => common::language::Language::Native,
            Self::Romaji => common::language::Language::Romaji,
        }
    }

    #[must_use]
    pub const fn from_common_language(value: &common::language::Language) -> Self {
        match value {
            common::language::Language::English => Self::English,
            common::language::Language::Native => Self::Native,
            common::language::Language::Romaji => Self::Romaji,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Calendar {
    #[serde(default)]
    pub id: i32,
    pub item_ids: Vec<i32>,
    pub language: Language,
    pub name: String,
    pub subscription_token: String,
    pub user_id: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    /// Per-calendar event style: `"timed"` (DTSTART:datetime) or
    /// `"all_day"` (DTSTART;VALUE=DATE). Phase 1 ships the column; Phase 2
    /// wires the toggle and consumer logic into `ics_export`.
    #[serde(default = "default_event_style")]
    pub event_style: String,
    /// Frozen .ics blob for Free-tier subscribe URLs. Server-managed: never
    /// deserialised from request bodies.
    #[serde(skip_deserializing, default)]
    pub frozen_subscribe_ics: Option<String>,
    /// Bumps when `name`, `language`, or `event_style` change on PUT.
    /// Used by Phase 3 SSE fan-out as a per-calendar revision cursor.
    /// Server-managed: never deserialised from request bodies.
    #[serde(skip_deserializing, default = "default_meta_version")]
    pub meta_version: i32,
}

impl Default for Calendar {
    fn default() -> Self {
        Self {
            id: 0,
            item_ids: Vec::new(),
            language: Language::default(),
            name: String::new(),
            subscription_token: String::new(),
            user_id: 0,
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
            event_style: default_event_style(),
            frozen_subscribe_ics: None,
            meta_version: default_meta_version(),
        }
    }
}

fn default_event_style() -> String {
    "timed".to_owned()
}

const fn default_meta_version() -> i32 {
    1
}
