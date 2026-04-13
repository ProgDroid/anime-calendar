use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct MediaCover {
    pub extra_large: String,
    pub large: String,
    pub medium: String,
    pub color: String,
}
