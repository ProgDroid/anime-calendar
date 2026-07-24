---
name: feedback_redis_numsub_fleet_subscriber_gate
description: "To gate work on \"is anyone watching across the fleet\" use Redis PUBSUB NUMSUB, not the local broadcast Sender::receiver_count"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ab83a158-ffc2-4fb3-a1dd-2857b860d545
---

To skip work whose only purpose is to populate an SSE frame nobody will receive (e.g. F2-31's actor-username `display` lookup in `add_item`/`remove_item`), gate on **`PUBSUB NUMSUB cal:{id}`** via `RedisPubSub::channel_subscriber_count`, exposed as `CalendarEventPublisher::calendar_has_subscribers` (fails *open* → `true` on Redis error).

**Why:** the app fans out one Redis `SUBSCRIBE` per channel per replica into in-process `tokio::broadcast` receivers (`redis_pubsub.rs`). `Sender::receiver_count()` therefore only counts SSE clients on the *current* replica — a subscriber on another replica would be missed, and that node would publish a frame with no `display`. `PUBSUB NUMSUB` counts every replica's SUBSCRIBE against the shared Redis, so non-zero = someone somewhere is watching. The correct global "is anyone listening?" gate.

**How to apply:** any future "only do X if a calendar is being watched" logic uses the publisher's `calendar_has_subscribers`, never the local receiver count. TOCTOU (a subscriber connecting between the NUMSUB check and the publish) is acceptable — they full-refetch on connect anyway. Related: [[feedback_sse_event_actor_filter]], [[feedback_cache_invalidation]].
