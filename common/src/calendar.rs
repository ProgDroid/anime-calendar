use crate::{id::Id, item::Item, language::Language};

pub struct Calendar {
    pub id: Id,
    pub items: Vec<Item>,
    pub language: Language,
    pub name: String, // TODO Name object?
}
