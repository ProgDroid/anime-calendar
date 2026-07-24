---
name: Data-source spike outcome — AniList stays (2026-05-05)
description: Spike to evaluate alternative anime data sources to AniList concluded — none of the 3 evaluated options (MAL, AnimeSchedule, Kitsu) are viable for our calendar use case. AniList remains the sole source.
type: project
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
The data-source spike that the monetisation plan flagged as a Phase 0 follow-up was completed. **Outcome: AniList stays as the sole anime data source.** No migration spec needed.

**Alternatives evaluated and why each failed:**

| Source | Blocker |
|--------|---------|
| **MyAnimeList (MAL) API** | Insufficient episode-level airing data. MAL exposes show-level info but not the per-episode airing schedule we depend on for the calendar's core value (date + time of next episode). |
| **AnimeSchedule.net** | Not available for commercial applications — license forbids commercial use. Anime Calendar's Pro tier ($2.99/$24.99) is a commercial product, so this rules them out. |
| **Kitsu** | Not available for commercial applications — same license-mode constraint as AnimeSchedule. |

**Why this matters going forward:**

1. **`AnimeDataSource` trait remains a vestigial generalisation.** Phase 0 introduced it (anilist/src/lib.rs) anticipating a future swap. The trait is still useful for caching/testing isolation, but the "swap to a better source" motivation is dead unless an unevaluated source emerges. Don't invest more abstraction work here without a new candidate to test.

2. **Spec 4 (MAL OAuth + list sync) is *not* killed by this** — list sync is "import what the user is tracking on MAL", which doesn't need MAL's airing schedule. The user's MAL-tracked shows would still resolve airing data via AniList. So Spec 4 stays viable; it just relies on AniList airing data even when the import source is MAL.

3. **Pricing model risk:** AniList is a single-vendor dependency. If AniList changes terms, rate-limits aggressively, or goes down for an extended window, our subscribe URLs degrade for everyone. Mitigations already in place (Phase 0 caching with split metadata/airing TTLs, frozen-blob fallback for Free, hourly poll cadence) buy time but don't replace the source.

4. **License clarity for future evaluators:** any new candidate must explicitly permit commercial use. Confirm this in writing before investing in a prototype — both AnimeSchedule and Kitsu were rejected at the license-read step.

**Date completed:** 2026-05-05.
