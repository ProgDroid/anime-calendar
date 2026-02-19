use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{id::Id, item::Item, language::Language};

#[derive(Deserialize, Serialize, Clone)]
pub struct Calendar {
    #[serde(default)]
    pub id: Id,
    pub items: Vec<Item>,
    pub language: Language,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
// TODO probably doesn't need to be a separate crate
