use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, sqlx::Type)]
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

#[derive(Clone, Default, Deserialize)]
pub struct Calendar {
    #[serde(default)]
    pub id: i32,
    pub item_ids: Vec<i32>,
    pub language: Language,
    pub name: String,
    pub user_id: i32,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}
