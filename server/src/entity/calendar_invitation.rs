use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Revoked,
    Expired,
    Suspended,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow, ToSchema)]
pub struct CalendarInvitation {
    pub id: i64,
    pub calendar_id: i32,
    pub inviter_id: i32,
    pub invitee_email: String,
    #[serde(skip)]
    pub token_hash: String,
    pub status: InvitationStatus,
    pub expires_at: NaiveDateTime,
    pub sent_at: NaiveDateTime,
    pub resolved_at: Option<NaiveDateTime>,
}
