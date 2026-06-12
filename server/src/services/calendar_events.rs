use chrono::NaiveDateTime;
use serde::Serialize;

use crate::{ServerResult, redis_pubsub::RedisPubSub};

/// Events published over the SSE / Redis Pub-Sub channel for a calendar.
///
/// # `actor` field contract
///
/// Every variant that carries an `actor` field MUST set it as follows:
/// - **Human-initiated events**: `user_id.to_string()` (the numeric database id,
///   e.g. `"42"`). Never the username.
/// - **Backend-initiated events** (webhook, reconcile): `"system".to_string()`.
///
/// The frontend self-echo filter that suppresses events originating from the
/// current user compares `frame.actor` to the authenticated user's id. Using
/// the username here breaks that filter silently.
///
/// `ItemAdded` / `ItemRemoved` additionally carry a `display` field: the
/// actor's human username (falling back to the numeric id string if the user
/// row can't be loaded). `display` is for rendering toasts; `actor` remains the
/// numeric id used by the self-echo filter. Do not conflate them.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CalendarEvent {
    ItemAdded {
        media_id: i32,
        actor: String,
        display: String,
        v: i32,
        at: NaiveDateTime,
    },
    ItemRemoved {
        media_id: i32,
        actor: String,
        display: String,
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

    /// Whether any SSE subscriber (on any replica) is currently watching this
    /// calendar's channel. Lets callers skip work whose only purpose is to
    /// populate an event payload nobody will receive (F2-31).
    ///
    /// Fails *open*: on a Redis error this returns `true`, so a transient
    /// `PUBSUB` hiccup never suppresses an event-enriching lookup.
    pub async fn calendar_has_subscribers(&self, calendar_id: i32) -> bool {
        self.pubsub
            .channel_subscriber_count(&format!("cal:{calendar_id}"))
            .await
            .map_or(true, |n| n > 0)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts that the `actor` field in every `CalendarEvent` variant that
    /// carries one serialises as the plain string that was passed in.
    /// This documents the wire-shape invariant: actor is always a `user_id`
    /// string (or "system") — never a username.
    #[test]
    fn actor_field_serialises_as_plain_string() {
        let actor = "42";
        let at = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let cases: &[(&str, CalendarEvent)] = &[
            (
                "ItemAdded",
                CalendarEvent::ItemAdded {
                    media_id: 1,
                    actor: actor.into(),
                    display: "Alice".into(),
                    v: 1,
                    at,
                },
            ),
            (
                "ItemRemoved",
                CalendarEvent::ItemRemoved {
                    media_id: 1,
                    actor: actor.into(),
                    display: "Alice".into(),
                    v: 1,
                    at,
                },
            ),
            (
                "MetaUpdated",
                CalendarEvent::MetaUpdated {
                    fields: vec![],
                    actor: actor.into(),
                    v: 1,
                    at,
                },
            ),
            (
                "MemberJoined",
                CalendarEvent::MemberJoined {
                    user_id: actor.into(),
                    display: "Alice".into(),
                    actor: actor.into(),
                },
            ),
            (
                "MemberLeft",
                CalendarEvent::MemberLeft {
                    user_id: actor.into(),
                    actor: actor.into(),
                    reason: "left".into(),
                },
            ),
        ];

        for (name, event) in cases {
            let json = serde_json::to_value(event).expect("serialize");
            assert_eq!(
                json["actor"],
                serde_json::Value::String(actor.to_string()),
                "{name}: actor must serialize as the exact string passed in"
            );
        }

        // The `display` field on item events must be emitted in the wire
        // payload — guards against a future `#[serde(skip)]` regression.
        let added = serde_json::to_string(&CalendarEvent::ItemAdded {
            media_id: 1,
            actor: actor.into(),
            display: "Alice".into(),
            v: 1,
            at,
        })
        .expect("serialize");
        assert!(
            added.contains("\"display\":\"Alice\""),
            "ItemAdded must serialize display: got {added}"
        );
    }

    /// Asserts that backend-initiated events use the "system" sentinel.
    #[test]
    fn system_actor_sentinel_serialises_correctly() {
        let event = CalendarEvent::MemberLeft {
            user_id: "42".into(),
            actor: "system".into(),
            reason: "suspended".into(),
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["actor"], serde_json::Value::String("system".into()));
    }
}
