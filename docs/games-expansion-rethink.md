# Rethink: extending anime-calendar to cover game releases

**Status:** parked. Nothing decided, nothing built.
**Date:** 2026-09-17

## The idea

Extend anime-calendar to also track game releases (IGDB as the data source
alongside AniList), possibly on separate subdomains or subfolders of the main
URL, since most of the functionality is the same with a different data source.
The original calendar idea was games — see the abandoned
[`game-calendar-tui`](https://github.com/ProgDroid/game-calendar-tui) repo.

## Where the detail lives

**Full write-up: `docs/anime-calendar-convergence.md` in the `game-calendar-tui`
repo.** That doc holds the verified findings, the open questions in order of
decisiveness, and the sequencing recommendation. This file is a pointer so the
idea is discoverable from this side; do not duplicate the content here.

## The short version of what it found

Three things verified against this codebase (`main` @ `926788e`) that shape any
future design:

1. **The "date firms up, event appears" mechanic works on Pro only.**
   `services/ics_export.rs` renders live per request, but `services/frozen_ics.rs`
   serves Free subscribers a blob frozen at last user edit. Anime tolerates that;
   games would not, since the whole point is a date changing while nobody is
   touching the calendar.
2. **There is no upstream change detection.** `services/reconcile.rs` is Stripe
   only. Schedule changes are found lazily on cache expiry, and nothing diffs old
   against new — so "this game slipped to 2028" is not expressible today.
3. **`calendar_items.item_id` has no source discriminator.** AniList 21 and
   IGDB 21 are the same row. Any games work starts with a migration adding one
   and backfilling `'anilist'` (migrations are up-only — see
   `docs/rollback-strategy.md`).

The seam itself is fine: `common::item::AnimeDataSource` is the right
abstraction, wrongly named. The work is generalising `common::Item` away from
its episode-shaped fields and making `IcsExportService` depend on the trait
rather than on `CachedAnilist` concretely.

## Sequencing

Not before the deployment lane ships, and ideally not before redesign
Tracks 2 / 4 / 3 land.
