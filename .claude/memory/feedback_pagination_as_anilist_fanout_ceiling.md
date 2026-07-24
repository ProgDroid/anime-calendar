---
name: Pagination on user-list endpoints is doing fan-out duty too
description: GET /calendars (and any sibling list endpoint that aggregates Anilist data per row) uses pagination to bound the worst-case Anilist batch size. Don't drop pagination without a replacement ceiling.
type: feedback
originSessionId: 76bbcb64-35b1-478f-a642-0b655f10cd65
---
**Rule:** Before dropping pagination on a user-list endpoint, audit whether it is silently bounding an Anilist (or other rate-limited upstream) batch. If yes, you cannot drop it without putting a replacement ceiling in place — even if the spec is silent on pagination.

**Why:** `GET /calendars` (and any sibling that fetches per-calendar items + airing data) does this:

1. Returns a paginated list of calendars.
2. Dedups the union of `item_ids` across the returned page.
3. Hits Anilist with one batched call covering the deduped set.
4. Derives `airing_count` (and similar) per calendar from the cached schedule map.

The default `page_size` is 6. With pagination, worst case is roughly `page_size × items_per_calendar`. Without pagination, a Pro power user with 50 calendars × 50 unique items would dump **2,500 ids in a single Anilist batch every cold-cache load** — an effectively self-inflicted rate-limit incident, since Anilist throttling is per-IP for the whole process.

The cache absorbs warm hits, but it cannot save us from cold-cache pathological cases. Pagination has been quietly providing the ceiling.

**How to apply:**

- When designing a new list endpoint that aggregates per-row data from Anilist or any other rate-limited upstream, **keep pagination** unless you put a hard cap somewhere else (per-user `LIMIT 100`, capped batch size, capped row count).
- When reshaping `GET /calendars` (or similar), default to keeping the existing pagination on the bounded list and adding any new lists as flat IF their size is naturally bounded by product rules. Spec 2's `shared_with_me` works as flat because it's bounded by `editor_cap × accepted_invitations`. Don't extrapolate that to lists without natural bounds.
- If the spec asks for an unpaginated list and there's no natural bound, push back with the fan-out concern before implementing. The user accepted `paginate owned + flat shared` after the elaboration.

**First seen:** Task 0.11 of co-editor-sharing (commit `952c47e`). User initially uncomfortable dropping pagination; elaborated the fan-out risk; landed on the split shape `{ owned: { data, pagination }, shared_with_me: [...] }`.

**Sites to scan when changing list endpoints:** `controllers/calendar.rs::get_calendars`, `services/cached_anilist.rs`, `cache.rs::generate_paginated_key`. The dedup + batch + derive pattern shows up there and likely in any future "list X with airing info" endpoint.
