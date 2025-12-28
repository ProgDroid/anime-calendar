use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>, // Optional for OAuth users
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
