use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Title {
    pub english: String,
    pub native: String,
    pub romaji: String,
}
