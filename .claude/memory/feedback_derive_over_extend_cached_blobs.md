---
name: Prefer deriving from cached blobs over extending them
description: Before adding fields to a cached entity (Item, etc.) to support a new aggregate, check if existing fields already encode the answer — saves cache-version bumps and cross-crate plumbing
type: feedback
originSessionId: 28f6b275-3872-4ade-932e-d785db439711
---
When a feature needs a new "X count / X status" aggregate on top of an already-cached entity (e.g. `common::Item` with its `airing_schedule`, `cover_image`, etc.), check whether existing fields encode the answer **before** adding a new field to the entity.

**Why:** During the 2026-05-01 airing-count work, the original plan (Path A) assumed adding `MediaStatus` to AniList query + `common::Item` + bumping the Redis cache key. That's ~3h of plumbing across 3 crates plus a cache invalidation event. Path B (derive `airing_count` in-memory from the existing `Item.airing_schedule` — "any future-dated `airing_at` ⇒ airing") shipped the same UX in ~half the time with zero cache churn, and the semantic was actually closer to user intent (a hiatus show with no upcoming episodes correctly reads as 0, where `MediaStatus::Releasing` would have over-counted it).

**How to apply:**
- When a plan calls for "add field X to `Item`/cached entity, bump cache version", first ask: *can I compute X from what's already there?*
- Pure derivation helpers (e.g. `count_airing(item_ids, schedules_by_id, now_secs) -> usize`) are unit-testable without standing up the AniList client. Keep the helper free of the heavy types — key the lookup map on `u64` / `&[Schedule]` rather than `&Item` so tests don't need to construct full `Item` instances.
- The cache-version-bump cost isn't just the code change — it's TTL-window of stale-blob garbage for every existing user. Avoiding it is a real win.
- Counter-rule: if the derivation is genuinely lossy (e.g. an item with no schedule entries at all could be either "not yet announced" or "finished") and the UX needs to distinguish, Path A is unavoidable. Path B works here because "no future episodes" is a fine UX answer for both states.
