---
name: Cache invalidation completeness
description: All cache keys derived from a mutated resource must be invalidated — including feed/subscription keys and derived renderings
type: feedback
originSessionId: 07abb350-f980-42a7-9b27-75b79b23a88f
---
When a calendar is mutated (PUT) or deleted, both the primary JSON key AND the iCal export key AND the subscription feed key must be invalidated. Originally only the primary key was cleared.

**Why:** Subscription feed served stale iCal data to external calendar clients (Google Calendar, Apple Calendar) for up to 1 hour after an update. Export cache was also silently broken (see: cache key collision below).

**How to apply:** On every mutation endpoint, audit ALL cache keys that are derived from the resource being changed — primary lookup, paginated lists, derived renderings (export, feed). Wire invalidation for each.

**Related production bug fixed 2026-04-15:** `invalidate_user_paged_calendars` was calling `self.delete(key)` where `key = "{user_id}:calendars:page:*"` — a literal key containing `*`, not a pattern scan. Redis `DEL` does not glob-expand; it deletes one exact key. The paginated calendar lists were therefore never cleared on mutation. Fixed to use `self.invalidate_pattern(&pattern)` which runs the `SCAN` cursor loop. The cache test suite now has a regression test: `invalidate_user_paged_calendars_removes_all_paginated_keys`.
