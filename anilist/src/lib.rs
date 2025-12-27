pub mod client;
pub mod error;
mod query;

pub use query::get_items::get_items::GetItemsPageMedia;
pub use query::get_items::get_items::MediaType as GetItemsMediaType;
pub use query::get_items::get_items::ResponseData as GetItemsResponseItem;
pub use query::search_items::search_items::MediaType as SearchItemsMediaType;
pub use query::search_items::search_items::ResponseData as SearchItemsResponseItem;
pub use query::search_items::search_items::SearchItemsPageMedia;
pub use query::search_items_by_type::search_items_by_type::MediaType as SearchItemsByTypeMediaType;
pub use query::search_items_by_type::search_items_by_type::ResponseData as SearchItemsByTypeResponseItem;
pub use query::search_items_by_type::search_items_by_type::SearchItemsByTypePageMedia;

use crate::error::AnilistError;
use std::result;

type Result<T> = result::Result<T, AnilistError>;
