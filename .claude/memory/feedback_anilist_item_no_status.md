---
name: common::Item lacks MediaStatus; cache uses serialized Item directly
description: Adding fields to Item invalidates Redis cache; status is not currently fetched from Anilist
type: feedback
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
The `Item` struct in `common/src/item.rs` does NOT currently have a `status` field (Anilist's `MediaStatus` enum: `FINISHED | RELEASING | NOT_YET_RELEASED | CANCELLED | HIATUS`). The `anilist` crate's GraphQL queries don't ask for it.

This matters because:

1. **Most "is this currently airing?" needs can be answered without `MediaStatus`** by checking `Item.airing_schedule` for any future-dated entry — see `feedback_derive_over_extend_cached_blobs.md` and the 2026-05-01 airing-count aggregate. Only reach for `MediaStatus` if you genuinely need to distinguish "finished" from "not yet announced" (both lack future episodes). When you do, extending the Anilist query AND the `Item` struct is required — the data isn't in the cache and never has been.

2. **The Redis cache holds serialized `Item` blobs directly.** Adding a non-optional field to `Item` invalidates every cached entry on deserialization. Two safe options when extending `Item`:
   - **(a)** Add `#[serde(default = "...")]` so old blobs deserialize with a fallback. Inaccurate for ~TTL window then self-corrects.
   - **(b)** Bump the cache key prefix/version (e.g. `item:v2:{id}`). Old blobs become orphaned and get GC'd by Redis TTL. Cleanest and recommended for any field where a default would lie.

3. **The Anilist client's `get_items(Vec<Id>)` is batched** via a single GraphQL query, so per-calendar aggregation is one round-trip per calendar (not N+1 per item). This makes server-side aggregates feasible — see the airing-count plan.

**How to apply:**
- If you need a new Anilist field, edit `anilist/src/query/` to add it to the GraphQL selection set, then `common/src/item.rs` to add the corresponding Rust field, then bump the cache version in `server/src/cache.rs`.
- Don't try to backfill via a separate "refresh" job — the architecture is "Anilist is source of truth, Redis is just a TTL'd response cache." TTL handles freshness.
- Per project memory `feedback_cache_invalidation.md`, any new derived cache key (e.g. `calendar:{id}:airing_count`) must be invalidated alongside the existing primary/export/subscription keys when the underlying resource mutates.
