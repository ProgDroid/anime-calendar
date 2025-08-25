use serde::{Deserialize, Serialize};

use crate::{id::Id, timestamp::Timestamp};

#[derive(Serialize, Deserialize)]
pub struct Schedule {
    pub id: Id,
    pub airing_at: Timestamp,
    pub episode: i64,
    pub media_id: Option<Id>,
}
