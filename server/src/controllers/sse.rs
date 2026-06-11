//! SSE endpoint: `GET /calendars/{id}/events`
//!
//! Streams calendar events to connected clients via Server-Sent Events.
//! On connection the handler:
//!   1. Authenticates the caller and asserts `ItemMutate` permission.
//!   2. Subscribes to `cal:{id}`, `presence:{id}`, and `kick:{actor_id}`
//!      broadcast channels backed by Redis Pub/Sub.
//!   3. Emits an initial `meta_snapshot` frame and the current presence list.
//!   4. Relays incoming channel messages and emits keepalive comments on a
//!      configurable interval.
//!   5. Closes the stream immediately after a kick message.

use std::{convert::Infallible, time::Duration};

use actix_web::{Responder, web};
use actix_web_lab::sse::{self, Sse};
use serde_json::json;

use crate::{
    config::server::SharingConfig,
    mappers::calendar::CalendarMapper,
    error::Error,
    middleware::auth::Claims,
    redis_pubsub::RedisPubSub,
    services::{
        calendar_events::{CalendarEvent, Viewer},
        presence::PresenceService,
        sharing_authz::{Action, SharingAuthz},
        sse_connection_tracker::SseConnectionTracker,
    },
};

/// Stream calendar events for a calendar the caller may view or edit.
///
/// Emits an initial `meta_snapshot` frame (current `meta_version`) and the
/// current presence list, then relays Redis Pub/Sub messages until the
/// connection closes or a `kick` event is received.
///
/// # Errors
/// - `401 Unauthorized` — no valid Bearer token.
/// - `403 Forbidden` — caller is neither owner nor active editor.
/// - `404 Not Found` — calendar does not exist or is soft-deleted.
/// - `429 Too Many Requests` — caller already has the per-user SSE cap of
///   open connections to this server.
/// - `500 Internal Server Error` — Redis subscription failure.
#[allow(clippy::future_not_send, clippy::too_many_arguments)]
pub async fn calendar_events(
    path: web::Path<i32>,
    claims: Claims,
    pool: web::Data<sqlx::PgPool>,
    authz: web::Data<SharingAuthz>,
    pubsub: web::Data<RedisPubSub>,
    presence: web::Data<PresenceService>,
    sharing_cfg: web::Data<SharingConfig>,
    tracker: web::Data<SseConnectionTracker>,
) -> Result<impl Responder, Error> {
    let calendar_id = path.into_inner();
    let actor_id = claims.user_id()?;

    // Load calendar and authorize.
    let cal = CalendarMapper::get_by_id_any_owner_from_pool(pool.get_ref(), calendar_id).await?;
    authz.assert_can(actor_id, &cal, Action::ItemMutate).await?;

    // Acquire a per-user SSE connection slot before subscribing to broadcast
    // channels. The guard decrements the counter on drop; we move it into the
    // stream closure below so it lives for the entire connection lifetime.
    let Some(connection_guard) = tracker.try_acquire(actor_id) else {
        return Err(Error::TooManyRequests);
    };

    // Subscribe to the three channels before building initial frames.
    let mut cal_rx = pubsub.subscribe(&format!("cal:{calendar_id}")).await?;
    let mut presence_rx = pubsub.subscribe(&format!("presence:{calendar_id}")).await?;
    let mut kick_rx = pubsub.subscribe(&format!("kick:{actor_id}")).await?;

    // Initial frames.
    let heartbeat_secs = sharing_cfg.sse_heartbeat_seconds;
    let meta_version = cal.meta_version;
    let viewers: Vec<Viewer> = presence.list(calendar_id).await.unwrap_or_default();

    let initial_meta = json!({ "type": "meta_snapshot", "v": meta_version }).to_string();
    let initial_presence = match serde_json::to_string(&CalendarEvent::Presence { viewers }) {
        Ok(s) => s,
        Err(e) => {
            log::error!("sse: failed to serialise initial presence frame: {e}");
            String::new()
        }
    };

    let stream = async_stream::stream! {
        // Hold the guard for the lifetime of the stream so the per-user
        // connection counter only decrements when the SSE connection closes.
        let _connection_guard = connection_guard;
        yield Ok::<sse::Event, Infallible>(sse::Event::Data(sse::Data::new(initial_meta)));
        yield Ok(sse::Event::Data(sse::Data::new(initial_presence)));

        let mut interval = tokio::time::interval(Duration::from_secs(u64::from(heartbeat_secs)));
        // Skip the immediately-firing first tick so we don't emit a spurious
        // comment right after the initial data frames.
        interval.tick().await;

        loop {
            tokio::select! {
                msg = cal_rx.recv() => match msg {
                    Ok(payload) => yield Ok(sse::Event::Data(sse::Data::new(payload))),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {} // skip and continue
                    // Closed means the fan-out sender is gone: recv() would
                    // resolve immediately on every select! iteration, spinning
                    // this task at full CPU while delivering nothing.
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        log::warn!("sse: cal channel closed, ending stream for calendar {calendar_id}");
                        break;
                    }
                },
                msg = presence_rx.recv() => match msg {
                    Ok(payload) => yield Ok(sse::Event::Data(sse::Data::new(payload))),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        log::warn!("sse: presence channel closed, ending stream for calendar {calendar_id}");
                        break;
                    }
                },
                msg = kick_rx.recv() => match msg {
                    Ok(payload) => {
                        yield Ok(sse::Event::Data(sse::Data::new(payload)));
                        break; // close stream after kick
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("sse: kick_rx lagged by {n} messages, closing stream as precaution");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        log::warn!("sse: kick channel closed, ending stream as precaution");
                        break;
                    }
                },
                _ = interval.tick() => {
                    yield Ok(sse::Event::Comment("ping".into()));
                }
            }
        }
    };

    Ok(Sse::from_stream(stream))
}

#[cfg(test)]
mod tests {
    use actix_web::{App, test, web};

    use crate::config::server::SharingConfig;
    use crate::services::sse_connection_tracker::SseConnectionTracker;

    /// Unauthenticated requests must be rejected with 401 without touching
    /// Redis or the database.
    ///
    /// This test exercises the auth extractor path only; it does not need a
    /// running Redis or Postgres instance.
    #[actix_web::test]
    async fn unauthenticated_request_returns_401() {
        let pool = crate::test_helpers::test_pool().await;

        // Build minimal Data stubs.  SharingAuthz, PresenceService, and
        // RedisPubSub are never reached on an unauthenticated request — the
        // Claims extractor short-circuits first.  We skip them here by relying
        // on the handler returning before any Data<T> extractor is called, but
        // actix-web requires all `web::Data<T>` types used in the handler
        // signature to be registered, or the handler returns 500 instead of 401.
        //
        // Register real instances where cheap (SharingConfig, PgPool).
        // For SharingAuthz and PresenceService — which require Redis — skip
        // registration and accept a 500 if the auth guard fires first.
        //
        // In practice the Claims extractor fires before any Data<T>, so the
        // 401 is returned without the handler body executing.

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(SharingConfig::default()))
                .app_data(web::Data::new(SseConnectionTracker::new(8)))
                .service(
                    web::resource("/calendars/{id}/events")
                        .route(web::get().to(super::calendar_events)),
                ),
        )
        .await;

        // The 429 cap-reached path is exercised by the SseConnectionTracker
        // unit tests in services/sse_connection_tracker.rs::tests. A full
        // integration test would require Redis + a seeded calendar + valid
        // Claims, which adds infrastructure that the deliberately-cheap
        // `unauthenticated_request_returns_401` test avoids. The cap logic
        // itself is fully unit-tested.

        let req = test::TestRequest::get()
            .uri("/calendars/1/events")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status().as_u16(),
            401,
            "expected 401 for unauthenticated SSE request"
        );
    }
}
