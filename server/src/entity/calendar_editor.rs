use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct CalendarEditor {
    pub calendar_id: i32,
    pub user_id: i32,
    pub active: bool,
    pub suspended_at: Option<NaiveDateTime>,
    pub joined_at: NaiveDateTime,
}
