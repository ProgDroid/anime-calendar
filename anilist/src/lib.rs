pub mod client;
pub mod error;
mod query;

pub use query::get_items::get_items::GetItemsPageMedia;
pub use query::get_items::get_items::MediaType as GetItemsMediaType;
pub use query::get_items::get_items::ResponseData as GetItemsResponseItem;

use crate::error::AnilistError;
use std::result;

type Result<T> = result::Result<T, AnilistError>;
