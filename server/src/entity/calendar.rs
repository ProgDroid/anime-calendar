use common::calendar::Calendar as CommonCalendar;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, sqlx::Type)]
#[sqlx(type_name = "language", rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Native,
    Romaji,
}

impl Language {
    pub const fn to_common_language(&self) -> common::language::Language {
        match self {
            Self::English => common::language::Language::English,
            Self::Native => common::language::Language::Native,
            Self::Romaji => common::language::Language::Romaji,
        }
    }

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
}

impl Calendar {
    #[allow(clippy::cast_possible_truncation)] // TODO fix
    pub fn from_common(calendar: &CommonCalendar) -> Self {
        let item_ids: Vec<i32> = calendar
            .items
            .iter()
            .map(|item| item.id.to_int() as i32)
            .collect();

        Self {
            id: calendar.id.to_int() as i32,
            item_ids,
            language: Language::from_common_language(&calendar.language),
            name: calendar.name.clone(),
        }
    }
}
