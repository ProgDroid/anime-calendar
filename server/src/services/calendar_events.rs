use chrono::NaiveDateTime;
use serde::Serialize;

use crate::{redis_pubsub::RedisPubSub, ServerResult};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CalendarEvent {
    ItemAdded {
        media_id: i32,
        actor: String,
        v: i32,
        at: NaiveDateTime,
    },
    ItemRemoved {
        media_id: i32,
        actor: String,
        v: i32,
        at: NaiveDateTime,
    },
    MetaUpdated {
        fields: Vec<String>,
        actor: String,
        v: i32,
        at: NaiveDateTime,
    },
    MemberJoined {
        user_id: String,
        display: String,
        actor: String,
    },
    MemberLeft {
        user_id: String,
        actor: String,
        reason: String,
    },
    Presence {
        viewers: Vec<Viewer>,
    },
    Kick {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Viewer {
    pub user_id: i32,
    pub display: String,
}

#[derive(Clone)]
pub struct CalendarEventPublisher {
    pubsub: RedisPubSub,
}

impl CalendarEventPublisher {
    #[must_use]
    pub fn new(pubsub: RedisPubSub) -> Self {
        Self { pubsub }
    }

    /// # Errors
    /// Propagates Redis publish errors.
    pub async fn publish_calendar(
        &self,
        calendar_id: i32,
        event: &CalendarEvent,
    ) -> ServerResult<()> {
        self.pubsub
            .publish(&format!("cal:{calendar_id}"), event)
            .await
    }

    /// # Errors
    /// Propagates Redis publish errors.
    pub async fn publish_kick(&self, user_id: i32, reason: &str) -> ServerResult<()> {
        self.pubsub
            .publish(
                &format!("kick:{user_id}"),
                &CalendarEvent::Kick {
                    reason: reason.into(),
                },
            )
            .await
    }

    /// # Errors
    /// Propagates Redis publish errors.
    pub async fn publish_presence(
        &self,
        calendar_id: i32,
        viewers: Vec<Viewer>,
    ) -> ServerResult<()> {
        self.pubsub
            .publish(
                &format!("presence:{calendar_id}"),
                &CalendarEvent::Presence { viewers },
            )
            .await
    }
}
