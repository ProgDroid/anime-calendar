use serde::{Deserialize, Serialize};

use crate::{id::Id, timestamp::Timestamp};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Schedule {
    #[cfg_attr(feature = "utoipa", schema(value_type = i64))]
    pub id: Id,
    #[cfg_attr(feature = "utoipa", schema(value_type = i64))]
    pub airing_at: Timestamp,
    pub episode: i64,
    #[cfg_attr(feature = "utoipa", schema(value_type = Option<i64>))]
    pub media_id: Option<Id>,
}
