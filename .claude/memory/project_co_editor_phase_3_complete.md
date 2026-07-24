---
name: Project: Co-editor sharing — Phase 3 complete (2026-05-07)
description: Live sync (SSE + Pub/Sub) shipped. 297 server / 450 frontend tests green. Phase 4 (tier transitions) is next.
type: project
originSessionId: 7da6ffde-691a-4b8c-a32d-d10222ee08c5
---
Phase 3 (Live Sync) of the co-editor sharing spec fully shipped on 2026-05-07.

**Commits:** `6bf8d28` (RedisPubSub) → `d558156` (Phase 3 verification gate). 8 tasks.

**What shipped:**
- `server/src/redis_pubsub.rs` — Pub/Sub wrapper with `MultiplexedConnection`, per-channel subscriber
- `server/src/services/event_publisher.rs` — `CalendarEventPublisher` (item add/remove, meta update, member join/leave, kick)
- `server/src/services/presence.rs` — heartbeat/list with Redis TTL keys + SCAN cursor loop
- `server/src/controllers/sse.rs` — `GET /calendars/{id}/events` SSE handler; subscribes to `cal:{id}`, `presence:{id}`, `kick:{user_id}`; initial snapshot; ping heartbeats; closes on kick
- `frontend/src/composables/usePresence.ts` — EventSource lifecycle, 30s heartbeat POST, dispatches all event types
- `frontend/src/components/shared/PresenceChip.vue` — viewer count chip + popover, self-detection by username
- Both editor views wired: `lastEvent` watcher, collision banner, `sharing.toasts.*` locale keys

**Tests:** 297 server / 450 frontend.

**Why:** Phase 4 (tier transitions) must come next — downgrade suspends editors + kicks live streams; upgrade restores. Cannot ship Phase 4 without Phase 3 live.

**How to apply:** Phase 4 plan is at `docs/superpowers/plans/2026-05-05-co-editor-sharing.md` Phase 4 section. Tasks 4.1–4.4 cover downgrade transaction, kick publish, frontend interrupt, and upgrade restore.
