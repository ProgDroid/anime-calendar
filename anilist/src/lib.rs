pub mod client;
pub mod error;
mod query;

pub use query::get_item::get_item::ResponseData as ResponseItem;

use crate::error::AnilistError;
use std::result;

type Result<T> = result::Result<T, AnilistError>;
