use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MediaCover {
    pub extra_large: String,
    pub large: String,
    pub medium: String,
    pub color: String,
}
