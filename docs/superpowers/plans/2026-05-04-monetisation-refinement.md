# Monetisation Refinement implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the marketing-only Pro feature list with an enforced tier model — items cap, calendar limit, frozen subscribe blob on Free, composable Pro reminders, per-calendar event style — and lower pricing to $2.99/mo · $24.99/yr. Foundation-first ordering (caching + `AnimeDataSource` trait) so subsequent phases benefit immediately.

**Architecture:** Approach A from the spec — Phase 0 lays the cache + trait foundation invisibly, Phase 1 adds tier enforcement and the frozen-blob subscribe path, Phase 2 layers reminders + event style on top, Phase 3 ships pricing + UpgradePage rewrite + locale cleanup. Each phase produces a working, committable state.

**Tech Stack:** Rust (Actix-Web 4 + sqlx 0.8 + redis 1.0 + icalendar 0.17), Vue 3.5 + TypeScript, Tailwind v4, Stripe via `async-stripe`. Existing Track 4 entitlement model reused unchanged.

**Date filed:** 2026-05-04
**Spec:** `docs/superpowers/specs/2026-05-04-monetisation-refinement-design.md`
**Companion checklist:** `docs/checklists/2026-05-monetisation-stripe-price-update.md`
**Estimated total effort:** ~2 weeks of focused work, 4 phases each independently shippable.

---

## Implementation status (last updated 2026-05-05)

| Phase | Status | Commits |
|-------|--------|---------|
| Pre-flight | ✅ Done | — |
| Phase 0 — Caching + AnimeDataSource trait | ✅ Done | `c122f47..1dfc40e` |
| Phase 1.1 — Schema + show count + entitlement caps + 402 reason | ✅ Done | `a6a5173` |
| Phase 1.2 — IcsExportService scaffold + FrozenIcsService | ✅ Done | `0ce59cf..b5aab57` |
| Phase 1.3 — PUT /calendar advisory lock + subscribe tier branching | ✅ Done | `a60bb38..212c93b` |
| Phase 1.4 — Stripe webhook frozen-blob lifecycle hooks | ✅ Done | `e3a8e00` |
| Phase 1.5 — Frontend counters + gates + 402 reason routing + /api/account/usage | ✅ Done | `07aa937..4c22f53` |
| Phase 2a (backend) — ics_export VALARM/event_style + settings/event_style validation | ✅ Done | `5f48997..f6a21d5` |
| Phase 2b (frontend) — Reminders chip-list + event_style toggle + locales | ✅ Done | this session |
| Phase 3 — Pricing reset + UpgradePage rewrite + locale cleanup | ✅ Done | this session |

**Pick-up notes for the next session:**
- Phase 2b entry points and Phase 3 entry points are catalogued in memory `project_monetisation_phase_2a_complete.md`.
- icalendar 0.17 outputs VALARM TRIGGERs in seconds form (`-PT1800S`); see memory `feedback_icalendar_duration_seconds_format`.
- Test baseline at end of 2026-05-04: 240 server lib + 3 anilist + 1 metrics integration + 392 frontend unit.
- The 402 reason-routing infra is already wired end-to-end; new cap gates plug into it (memory `reference_402_reason_routing`).

---

## File map

**Backend — new:**
- `server/src/services/cached_data_source.rs` — generic caching adapter
- `server/src/services/show_count.rs` — distinct-shows counter
- `server/src/services/frozen_ics.rs` — frozen blob writer
- `server/src/services/ics_export.rs` — centralised .ics rendering (refactor target)

**Backend — modified:**
- `anilist/src/lib.rs` — define `AnimeDataSource` trait, refactor existing client as one impl
- `server/src/cache.rs` — add per-item meta+airing keys, drop batch key, config-driven TTLs
- `server/src/services/entitlement.rs` — add `assert_can_create_calendar`, `assert_can_add_show`
- `server/src/controllers/calendar.rs` — wire entitlement checks; subscribe-endpoint tier branching
- `server/src/controllers/stripe.rs` — webhook hooks for frozen-blob lifecycle
- `server/src/entity/calendar.rs`, `server/src/entity/user_settings.rs` — new columns
- `server/src/main.rs` — DI wiring updates
- `config.toml`, `config.toml.dist`, `Config` struct — `[cache]`, `[limits]` blocks

**Backend — migrations:**
- `server/migrations/<n>_add_calendar_event_style.sql`
- `server/migrations/<n>_add_calendar_frozen_subscribe_ics.sql`
- `server/migrations/<n>_add_user_settings_reminder_offsets.sql`

**Frontend — modified:**
- `frontend/src/components/UpgradePage.vue`
- `frontend/src/components/MyCalendarsPage.vue`
- `frontend/src/components/calendar/EditorItemsPanel.vue` (+ mobile variant)
- `frontend/src/components/account/PreferencesTab.vue`
- `frontend/src/components/calendar/CalendarSettingsForm.vue`
- `frontend/src/stores/userSettingsStore.ts`
- `frontend/src/services/calendars.ts`, `frontend/src/services/userSettingsService.ts`
- `frontend/src/locales/en.json`, `frontend/src/locales/pt.json`

---

## Pre-flight (before Phase 0)

- [ ] Verify `subscribe_token` on `calendars` is uniquely indexed. If not, add the index in Phase 1's migration batch. Check via `\d calendars` in psql.
- [x] Confirm migration numbering: list `server/migrations/` and pick the next three sequential numbers.
- [x] Decide config rollout: copy current `config.toml`, append the new `[cache]` and `[limits]` sections with the spec's default values. Ship in Phase 0 so Phase 1 has them available.
- [x] Run baseline `cargo test --workspace` and `cd frontend && npm run test:unit` — capture green output for diff comparison after each phase.

---

## Phase 0 — Caching foundation + `AnimeDataSource` trait

**Goal:** invisible refactor. Define the data-source trait, add a caching adapter that splits metadata vs airing TTLs, drop the now-redundant batch cache key. App behaves identically from the user's perspective; backend cache hit rate improves on cross-user overlap.

### Tasks

#### Trait + impl

- [x] Define `common::AnimeDataSource` in `anilist/src/lib.rs` (or new top-level module — pick whichever fits the existing crate layout):

```rust
#[async_trait::async_trait]
pub trait AnimeDataSource: Send + Sync {
    async fn search(
        &self,
        query: &str,
        media_type: Option<MediaType>,
    ) -> anilist::Result<Vec<common::Item>>;

    async fn fetch_by_ids(
        &self,
        ids: &[common::id::Id],
    ) -> anilist::Result<Vec<common::Item>>;

    async fn fetch_airing_schedule(
        &self,
        id: common::id::Id,
    ) -> anilist::Result<Vec<common::AiringEpisode>>;
}
```

If `MediaType` / `AiringEpisode` aren't already shared types, use the existing AniList-specific shapes in this trait — refactoring to a generic shape is for the data-source spike, not this phase.

- [x] Add `async_trait = "0.1"` to `anilist/Cargo.toml` if not present.
- [x] Implement `AnimeDataSource` for the existing AniList client struct. The `impl` block is mostly delegation to existing methods; rename the existing methods if the trait signature requires it, but preserve external behaviour.
- [x] Existing tests in `anilist/` should still pass unchanged.

#### Cache key changes

- [x] In `server/src/cache.rs`: drop `generate_items_key` (the batch key — `pub fn generate_items_key(ids: &[common::id::Id]) -> String`). Search for callers and remove. Anything that currently calls it should be migrated to fetch via the cached adapter we're about to introduce.
- [x] Add new key generators:

```rust
#[must_use]
pub fn generate_item_meta_key(id: i64) -> String {
    format!("item:meta:{id}")
}

#[must_use]
pub fn generate_item_airing_key(id: i64) -> String {
    format!("item:airing:{id}")
}
```

- [x] Drop the existing `generate_item_key` (it served the old combined-blob shape). All consumers will migrate to the new pair.

#### Config plumbing

- [x] Add to `config.toml.dist` and `config.toml`:

```toml
[cache]
metadata_ttl_seconds       = 86400   # 24h — static AniList fields (title, description, etc.)
airing_ttl_seconds         = 900     # 15m — airing schedule (next-episode time)
search_ttl_seconds         = 3600    # 1h — search results
calendar_items_ttl_seconds = 300     # 5m — resolved items per calendar
export_ttl_seconds         = 300     # 5m — rendered .ics

[limits]
free_calendar_limit = 3
free_show_cap       = 25
pro_max_reminders   = 5
```

- [x] Extend the `Config` struct in `server/src/config.rs` (or wherever it lives) with `CacheConfig` and `LimitsConfig` sub-structs. Add `Deserialize`. Wire into the existing `Config::load()` path. Add a default-fallback or fail-fast if missing — match the codebase's existing convention (Track 4's Stripe section was fail-fast; do the same).
- [x] Verify `cargo build` clean after config changes.

#### Cached adapter

- [x] Create `server/src/services/cached_data_source.rs`. Sketch:

```rust
use anilist::{AnimeDataSource, MediaType};
use common::{id::Id, AiringEpisode, Item};
use std::collections::HashMap;

#[derive(Clone)]
pub struct CachedDataSource<S: AnimeDataSource + Clone> {
    inner: S,
    cache: crate::cache::Cache,
    metadata_ttl: u64,
    airing_ttl: u64,
    search_ttl: u64,
}

impl<S: AnimeDataSource + Clone> CachedDataSource<S> {
    #[must_use]
    pub const fn new(
        inner: S,
        cache: crate::cache::Cache,
        ttls: &crate::config::CacheConfig,
    ) -> Self {
        Self {
            inner,
            cache,
            metadata_ttl: ttls.metadata_ttl_seconds,
            airing_ttl: ttls.airing_ttl_seconds,
            search_ttl: ttls.search_ttl_seconds,
        }
    }
}

#[async_trait::async_trait]
impl<S: AnimeDataSource + Clone + Send + Sync> AnimeDataSource for CachedDataSource<S> {
    async fn search(
        &self,
        query: &str,
        media_type: Option<MediaType>,
    ) -> anilist::Result<Vec<Item>> {
        let key = crate::cache::generate_search_key(
            query,
            media_type.as_ref().map(MediaType::as_str),
        );
        // get_or_set pattern — existing helper in cache.rs
        self.cache
            .get_or_set(&key, self.search_ttl, || self.inner.search(query, media_type))
            .await
    }

    async fn fetch_by_ids(&self, ids: &[Id]) -> anilist::Result<Vec<Item>> {
        // 1. Per-id cache check — both meta + airing must hit
        let mut cached: HashMap<Id, Item> = HashMap::with_capacity(ids.len());
        let mut missing: Vec<Id> = Vec::new();
        for &id in ids {
            let meta_key = crate::cache::generate_item_meta_key(id.to_int());
            let airing_key = crate::cache::generate_item_airing_key(id.to_int());
            let meta: Option<ItemMeta> = self.cache.get(&meta_key).await.unwrap_or(None);
            let airing: Option<ItemAiring> = self.cache.get(&airing_key).await.unwrap_or(None);
            match (meta, airing) {
                (Some(m), Some(a)) => {
                    cached.insert(id, Item::from_split(m, a));
                }
                _ => missing.push(id),
            }
        }

        // 2. Batch upstream for missing
        if !missing.is_empty() {
            let fetched = self.inner.fetch_by_ids(&missing).await?;
            for item in &fetched {
                let (meta, airing) = item.clone().into_split();
                let id = item.id;
                let _ = self
                    .cache
                    .set(
                        &crate::cache::generate_item_meta_key(id.to_int()),
                        &meta,
                        self.metadata_ttl,
                    )
                    .await;
                let _ = self
                    .cache
                    .set(
                        &crate::cache::generate_item_airing_key(id.to_int()),
                        &airing,
                        self.airing_ttl,
                    )
                    .await;
                cached.insert(id, item.clone());
            }
        }

        // 3. Reassemble in caller order
        Ok(ids.iter().filter_map(|id| cached.remove(id)).collect())
    }

    async fn fetch_airing_schedule(&self, id: Id) -> anilist::Result<Vec<AiringEpisode>> {
        // No new key needed — airing schedule is derived from the item already cached.
        // Pass through to the inner source for now; if profiling shows hot, we can add
        // a dedicated key later.
        self.inner.fetch_airing_schedule(id).await
    }
}
```

- [x] `Item::into_split() -> (ItemMeta, ItemAiring)` and `Item::from_split(meta, airing) -> Item` need to live somewhere — likely `common/src/item.rs`. Define `ItemMeta` (title, episodes, format, status, etc. — the slow-changing fields) and `ItemAiring` (airing_at, next-episode, etc. — the fast-changing fields) as the two halves. Make both `Serialize + Deserialize + Clone`.

  If splitting `Item` is intrusive, an acceptable v1 alternative is to cache the whole `Item` under both keys (effectively duplicating storage but using the dual TTL gate). Pick the cleaner option in code review; the test suite should cover both possibilities.

- [x] DI in `server/src/main.rs`: replace the existing `web::Data::new(AniListClient::new(...))` with:

```rust
let cached_data_source = CachedDataSource::new(
    AniListClient::new(...),
    cache.clone(),
    &config.cache,
);
let cached_data_source: web::Data<dyn AnimeDataSource> =
    web::Data::from(Arc::new(cached_data_source) as Arc<dyn AnimeDataSource>);
app.app_data(cached_data_source.clone());
```

If using `dyn` boxing causes lifetime/Send/Sync friction with Actix's `web::Data`, fall back to a concrete type alias `pub type AnimeData = CachedDataSource<AniListClient>;` and inject that. Both work; the trait-object path is cleaner for future swaps but only matters once a second source impl exists.

- [x] Update controllers that previously took `web::Data<AniListClient>` to take `web::Data<dyn AnimeDataSource>` (or the alias). No behaviour change — same method calls.

#### Tests

- [x] Test helper: `MockAnimeDataSource` in `server/src/services/cached_data_source.rs` `#[cfg(test)] mod tests {}`. A struct with `responses: HashMap<Id, Item>` and a `panic_on_call: bool` flag. Implement `AnimeDataSource` to return preconfigured responses. Used across cache tests.

- [x] Tests inline in `cached_data_source.rs`:

```rust
#[tokio::test]
async fn cached_returns_from_cache_when_both_keys_present() {
    let cache = build_test_cache().await;
    cache.set(&generate_item_meta_key(42), &test_meta(), 300).await.unwrap();
    cache.set(&generate_item_airing_key(42), &test_airing(), 300).await.unwrap();
    let mock = MockAnimeDataSource::panicking();
    let cds = CachedDataSource::new(mock, cache, &test_ttls());
    let items = cds.fetch_by_ids(&[Id::from(42)]).await.unwrap();
    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn partial_miss_only_fetches_missing_ids() {
    let cache = build_test_cache().await;
    cache.set(&generate_item_meta_key(42), &test_meta(), 300).await.unwrap();
    cache.set(&generate_item_airing_key(42), &test_airing(), 300).await.unwrap();
    let mock = MockAnimeDataSource::with_responses(vec![(Id::from(99), test_item(99))])
        .expect_call_with_ids(vec![Id::from(99)]);
    let cds = CachedDataSource::new(mock, cache, &test_ttls());
    let items = cds.fetch_by_ids(&[Id::from(42), Id::from(99)]).await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, Id::from(42));
    assert_eq!(items[1].id, Id::from(99));
}

#[tokio::test]
async fn redis_failure_falls_back_to_upstream() {
    let broken_cache = build_failing_cache();
    let mock = MockAnimeDataSource::with_responses(vec![(Id::from(7), test_item(7))]);
    let cds = CachedDataSource::new(mock, broken_cache, &test_ttls());
    let items = cds.fetch_by_ids(&[Id::from(7)]).await.unwrap();
    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn result_order_matches_caller_order() {
    let cache = build_test_cache().await;
    let mock = MockAnimeDataSource::with_responses(vec![
        (Id::from(3), test_item(3)),
        (Id::from(1), test_item(1)),
        (Id::from(2), test_item(2)),
    ]);
    let cds = CachedDataSource::new(mock, cache, &test_ttls());
    let items = cds
        .fetch_by_ids(&[Id::from(3), Id::from(1), Id::from(2)])
        .await
        .unwrap();
    assert_eq!(items[0].id, Id::from(3));
    assert_eq!(items[1].id, Id::from(1));
    assert_eq!(items[2].id, Id::from(2));
}
```

- [x] TTL-separation test: harder to write without time mocking. Alternative — write a behaviour test that sets meta/airing keys with different TTLs (300s vs 1s), sleeps 2s, asserts that the next fetch only triggers an upstream call (because airing is expired but meta still cached). This is a real-time test; mark `#[ignore]` if it's flaky and run manually. Skip to a follow-up if it adds noise to CI.

### Acceptance

- `AWS_LC_SYS_PREBUILT_NASM=1 cargo build --workspace` clean.
- `cargo test --workspace` — all green, including new `cached_data_source` tests.
- `cargo clippy --workspace -- -D warnings` clean.
- Frontend untouched; `cd frontend && npm run build && npm run test:unit` still passes.
- App boots with new `[cache]` and `[limits]` config sections; manual smoke test of search + calendar load shows no behaviour change.

### Suggested commits

1. `feat(anilist): introduce AnimeDataSource trait + impl on existing client`
2. `feat(common): split Item into Meta + Airing halves` (if pursuing the split-type approach)
3. `feat(cache): add per-item meta/airing keys, drop batch key, config-driven TTLs`
4. `feat(server): CachedDataSource adapter with split-TTL writeback`
5. `chore(server): wire cached data source into DI; replace direct AniList injection`

---

## Phase 1 — Tier infrastructure (cap enforcement + frozen blob)

**Goal:** server-side enforcement of free-tier caps lands. Subscribe URL becomes tier-aware. Frontend surfaces counters and upgrade modals at the right friction points. App now actually behaves differently for Free vs Pro across the headline gates.

### Tasks

#### Schema

- [x] sqlx migration `<n>_add_calendar_event_style.sql`:

```sql
ALTER TABLE calendars
    ADD COLUMN event_style TEXT NOT NULL DEFAULT 'timed'
    CHECK (event_style IN ('timed', 'all_day'));
```

- [x] sqlx migration `<n+1>_add_calendar_frozen_subscribe_ics.sql`:

```sql
ALTER TABLE calendars
    ADD COLUMN frozen_subscribe_ics TEXT;
```

- [x] sqlx migration `<n+2>_add_user_settings_reminder_offsets.sql`:

```sql
ALTER TABLE user_settings
    ADD COLUMN reminder_offsets_minutes INTEGER[] NOT NULL DEFAULT ARRAY[30];
```

- [x] If `subscribe_token` is not already uniquely indexed, also add:

```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_calendars_subscribe_token
    ON calendars(subscribe_token);
```

- [x] Run `DATABASE_URL=... cargo sqlx prepare --workspace -- --all-targets` after queries are written; commit `.sqlx/` (per memory `feedback_sqlx_offline_cache`).

#### Entity updates

- [x] `server/src/entity/calendar.rs`: add `event_style: String`, `frozen_subscribe_ics: Option<String>` fields to the `Calendar` struct. Update any `FromRow` / serialization derives. If there's a `CalendarPayload` separate from `Calendar`, add `event_style` to the payload too.
- [x] `server/src/entity/user_settings.rs`: add `reminder_offsets_minutes: Vec<i32>`. Update `FromRow`. If serde uses `#[serde(default)]` somewhere, set the default to `vec![30]` to match the SQL default.

#### Show count service

- [x] Create `server/src/services/show_count.rs`:

```rust
use sqlx::PgPool;

#[derive(Clone)]
pub struct ShowCountService {
    pool: PgPool,
}

impl ShowCountService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Count distinct media IDs across all of this user's calendars.
    /// One AniList call per distinct ID is the cost we're metering.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn count_distinct_for_user(&self, user_id: i32) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(DISTINCT ci.media_id) AS "count!"
               FROM calendar_items ci
               JOIN calendars c ON c.id = ci.calendar_id
               WHERE c.owner_id = $1"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Returns true if the given media_id is already tracked by the user
    /// in any of their calendars. Used by the "idempotent add" check.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn user_already_tracks(&self, user_id: i32, media_id: i64) -> sqlx::Result<bool> {
        sqlx::query_scalar!(
            r#"SELECT EXISTS(
                 SELECT 1 FROM calendar_items ci
                 JOIN calendars c ON c.id = ci.calendar_id
                 WHERE c.owner_id = $1 AND ci.media_id = $2
               ) AS "exists!""#,
            user_id,
            media_id
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Count calendars owned by the user.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn count_calendars_for_user(&self, user_id: i32) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!" FROM calendars WHERE owner_id = $1"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
    }
}
```

- [x] Inline tests:

```rust
#[sqlx::test]
async fn count_distinct_excludes_duplicates(pool: PgPool) {
    let user = make_user(&pool).await;
    let cal_a = make_calendar(&pool, user.id).await;
    let cal_b = make_calendar(&pool, user.id).await;
    add_item(&pool, cal_a.id, /* media_id */ 100).await;
    add_item(&pool, cal_a.id, 200).await;
    add_item(&pool, cal_b.id, 100).await; // duplicate
    let svc = ShowCountService::new(pool.clone());
    assert_eq!(svc.count_distinct_for_user(user.id).await.unwrap(), 2);
}

#[sqlx::test]
async fn count_distinct_per_user_isolation(pool: PgPool) {
    let user_a = make_user(&pool).await;
    let user_b = make_user(&pool).await;
    let cal = make_calendar(&pool, user_a.id).await;
    add_item(&pool, cal.id, 100).await;
    let svc = ShowCountService::new(pool.clone());
    assert_eq!(svc.count_distinct_for_user(user_a.id).await.unwrap(), 1);
    assert_eq!(svc.count_distinct_for_user(user_b.id).await.unwrap(), 0);
}

#[sqlx::test]
async fn user_already_tracks_returns_true_for_any_calendar(pool: PgPool) {
    let user = make_user(&pool).await;
    let cal_a = make_calendar(&pool, user.id).await;
    let cal_b = make_calendar(&pool, user.id).await;
    add_item(&pool, cal_a.id, 42).await;
    let svc = ShowCountService::new(pool.clone());
    assert!(svc.user_already_tracks(user.id, 42).await.unwrap());
    assert!(!svc.user_already_tracks(user.id, 99).await.unwrap());
}
```

#### Entitlement extensions

- [x] Add to `server/src/services/entitlement.rs`:

```rust
use crate::services::show_count::ShowCountService;

#[derive(Clone)]
pub struct EntitlementService {
    mapper: SubscriptionMapper,
    show_count: ShowCountService,
    free_calendar_limit: i64,
    free_show_cap: i64,
}

impl EntitlementService {
    #[must_use]
    pub const fn new(
        mapper: SubscriptionMapper,
        show_count: ShowCountService,
        limits: &crate::config::LimitsConfig,
    ) -> Self {
        Self {
            mapper,
            show_count,
            free_calendar_limit: limits.free_calendar_limit as i64,
            free_show_cap: limits.free_show_cap as i64,
        }
    }

    // existing effective_tier / entitlement methods unchanged

    /// Returns Ok if the user can create another calendar at their tier.
    /// Pro: always Ok. Free: blocked at `free_calendar_limit`.
    ///
    /// # Errors
    /// Returns `Error::PaymentRequired` if the cap would be exceeded,
    /// or a database error if the count query fails.
    pub async fn assert_can_create_calendar(&self, user_id: i32) -> ServerResult<()> {
        if matches!(self.effective_tier(user_id).await?, Tier::Paid) {
            return Ok(());
        }
        let count = self.show_count.count_calendars_for_user(user_id).await?;
        if count >= self.free_calendar_limit {
            return Err(Error::PaymentRequired {
                required_tier: "paid",
                reason: Some("cap_calendars"),
            });
        }
        Ok(())
    }

    /// Returns Ok if the user can add `media_id` (which may or may not
    /// already be tracked). Idempotent on already-tracked IDs.
    ///
    /// # Errors
    /// Returns `Error::PaymentRequired` when adding a *new* media_id while
    /// the user is at the free cap. Returns Ok if the user already tracks
    /// the media_id anywhere (count unchanged) or is on Pro.
    pub async fn assert_can_add_show(&self, user_id: i32, media_id: i64) -> ServerResult<()> {
        if matches!(self.effective_tier(user_id).await?, Tier::Paid) {
            return Ok(());
        }
        if self.show_count.user_already_tracks(user_id, media_id).await? {
            return Ok(()); // idempotent
        }
        let count = self.show_count.count_distinct_for_user(user_id).await?;
        if count >= self.free_show_cap {
            return Err(Error::PaymentRequired {
                required_tier: "paid",
                reason: Some("cap_shows"),
            });
        }
        Ok(())
    }
}
```

- [x] Inline tests:

```rust
#[sqlx::test]
async fn assert_can_create_calendar_pro_passes(pool: PgPool) { /* Pro user with 50 cals → Ok */ }

#[sqlx::test]
async fn assert_can_create_calendar_free_under_cap_passes(pool: PgPool) {
    /* Free with 2/3 → Ok */
}

#[sqlx::test]
async fn assert_can_create_calendar_free_at_cap_blocks(pool: PgPool) {
    /* Free with 3/3 → Err(PaymentRequired) */
}

#[sqlx::test]
async fn assert_can_add_show_idempotent_on_already_tracked(pool: PgPool) {
    /* Free at cap, but adding a media_id that's already in another of their calendars → Ok */
}

#[sqlx::test]
async fn assert_can_add_show_blocks_new_media_at_cap(pool: PgPool) {
    /* Free with 25/25, new media_id → Err */
}

#[sqlx::test]
async fn assert_can_add_show_pro_unlimited(pool: PgPool) {
    /* Pro with 500 shows → Ok */
}
```

#### Frozen ICS service

- [x] Refactor existing inline .ics rendering into `server/src/services/ics_export.rs` first if not already done (this is the prep that Phase 2 also depends on). Expose at least:

```rust
pub struct IcsExportService { /* ... */ }

impl IcsExportService {
    /// Render a calendar as a complete .ics document. Pulls items from DB,
    /// resolves AniList data via the cached data source, applies the
    /// owner's reminder offsets and the calendar's event style.
    ///
    /// # Errors
    /// Returns errors for DB / cache / AniList failures.
    pub async fn render(&self, calendar_id: i32) -> ServerResult<String> { /* ... */ }
}
```

For Phase 1, the reminders portion can hardcode a single 30-min `VALARM` for everyone (Phase 2 makes it tier-aware). The `event_style` portion can hardcode `'timed'` (Phase 2 wires the toggle). The point is to centralise the rendering.

- [x] Create `server/src/services/frozen_ics.rs`:

```rust
use sqlx::PgPool;
use crate::services::ics_export::IcsExportService;

#[derive(Clone)]
pub struct FrozenIcsService {
    pool: PgPool,
    export: IcsExportService,
}

impl FrozenIcsService {
    #[must_use]
    pub const fn new(pool: PgPool, export: IcsExportService) -> Self {
        Self { pool, export }
    }

    /// Generate the frozen .ics for a single calendar and write it
    /// to `calendars.frozen_subscribe_ics`. Idempotent.
    ///
    /// # Errors
    /// Propagates rendering or DB errors. Caller should log and continue
    /// when used in batch (`regenerate_for_user`).
    pub async fn regenerate(&self, calendar_id: i32) -> ServerResult<()> {
        let blob = self.export.render(calendar_id).await?;
        sqlx::query!(
            "UPDATE calendars SET frozen_subscribe_ics = $1 WHERE id = $2",
            blob,
            calendar_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Regenerate frozen blobs for every calendar owned by the user.
    /// Per-calendar failures are logged but do not abort the loop.
    ///
    /// # Errors
    /// Returns the number of failures so callers can decide whether
    /// to alert.
    pub async fn regenerate_for_user(&self, user_id: i32) -> ServerResult<usize> {
        let calendar_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT id FROM calendars WHERE owner_id = $1",
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;
        let mut failures = 0;
        for id in calendar_ids {
            if let Err(e) = self.regenerate(id).await {
                log::error!("frozen_ics regenerate failed for calendar {id}: {e:?}");
                failures += 1;
            }
        }
        Ok(failures)
    }

    /// Null all frozen blobs for the user. Called on Free → Pro transition.
    ///
    /// # Errors
    /// Database error.
    pub async fn clear_for_user(&self, user_id: i32) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE calendars SET frozen_subscribe_ics = NULL WHERE owner_id = $1",
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

- [x] Inline tests:

```rust
#[sqlx::test]
async fn regenerate_writes_current_state(pool: PgPool) {
    /* setup user + calendar + items, call regenerate, assert column non-null and contains expected lines */
}

#[sqlx::test]
async fn regenerate_is_idempotent(pool: PgPool) {
    /* call regenerate twice, assert column is identical to a single-call run */
}

#[sqlx::test]
async fn regenerate_for_user_does_not_abort_on_one_calendar_failure(pool: PgPool) {
    /* mock data source that fails for calendar B's items; assert calendar A's blob populated, B's null, returns 1 failure */
}

#[sqlx::test]
async fn clear_for_user_nulls_all_blobs(pool: PgPool) {
    /* populate blobs across 3 calendars, call clear_for_user, assert all null */
}
```

#### Controller wiring — `PUT /calendar`

- [x] In `server/src/controllers/calendar.rs`, the `put` handler. Determine whether the request is creating a new calendar (no `id` in payload) or updating an existing one (id present). For the create branch:

```rust
// at the top of the create branch, before insert
entitlement.assert_can_create_calendar(user_id).await?;
```

- [x] For the items-diff branch (whether create or update), iterate over the items in the payload and call `assert_can_add_show` for each:

```rust
for item in &payload.items {
    entitlement.assert_can_add_show(user_id, item.media_id).await?;
}
```

Idempotent already-tracked IDs return Ok immediately; only newly-introduced IDs at the cap trigger 402.

- [x] Wrap the count-then-insert in a transaction with a per-user advisory lock to prevent the multi-tab race:

```rust
let mut tx = pool.begin().await?;
sqlx::query!("SELECT pg_advisory_xact_lock($1)", user_id as i64)
    .execute(&mut *tx)
    .await?;
// re-run entitlement checks here with the locked view (the earlier checks
// outside the lock are best-effort fast-path; the locked re-check is
// authoritative)
entitlement.assert_can_create_calendar(user_id).await?; // for create branch
for item in &payload.items {
    entitlement.assert_can_add_show(user_id, item.media_id).await?;
}
// existing INSERT / UPDATE logic, using `&mut *tx`
tx.commit().await?;
```

`pg_advisory_xact_lock` is per-connection scoped to the transaction and serialises only when called with the same key; user_id is a clean key. Other users' requests are unaffected.

- [x] After the transactional commit, if owner is Free, call `frozen_ics.regenerate(calendar_id)`. Log errors but don't fail the response — the safety-net path on subscribe poll covers any holes.

- [x] Inline integration test:

```rust
#[sqlx::test]
async fn put_calendar_create_at_cap_returns_402_free(pool: PgPool) {
    let app = test_app(pool.clone()).await;
    let user = make_free_user_with_calendars(&pool, 3).await; // at limit
    let response = app
        .call(test::TestRequest::put().uri("/api/calendar")
            .set_json(json!({"title": "fourth", "items": []}))
            .auth(&user)
            .to_request())
        .await;
    assert_eq!(response.status(), 402);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["required_tier"], "paid");
}

#[sqlx::test]
async fn add_item_at_cap_already_tracked_succeeds_free(pool: PgPool) {
    /* Free user at 25-show cap, second calendar puts in an already-tracked media_id → 200 */
}

#[sqlx::test]
async fn add_item_at_cap_new_media_returns_402(pool: PgPool) {
    /* Free user at cap, new media_id → 402 */
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_add_at_cap_one_wins_one_402() {
    /* Spawn two concurrent tasks both attempting to add new shows to a Free user at cap-1.
       Use barriers to align them. Assert exactly one returns 200, exactly one returns 402. */
}
```

#### Controller wiring — subscribe endpoint

- [x] Update `subscribe_feed` (line ~300 in `calendar.rs`):

```rust
let cal = mapper.find_by_token(&token).await?;
let cal = match cal {
    Some(c) => c,
    None => return Err(Error::NotFound), // existing 404 path
};

let owner_tier = entitlement.effective_tier(cal.owner_id).await?;
match owner_tier {
    Tier::Paid => {
        // existing live path — render via ics_export with caching
        let blob = ics_export.render(cal.id).await?;
        return Ok(HttpResponse::Ok()
            .content_type("text/calendar; charset=utf-8")
            .body(blob));
    }
    Tier::Free => {
        if let Some(blob) = cal.frozen_subscribe_ics {
            return Ok(HttpResponse::Ok()
                .content_type("text/calendar; charset=utf-8")
                .body(blob));
        }
        // Column null while Free — disambiguate "never Pro" vs "webhook delayed"
        let has_history = subscription_mapper.exists_for_user(cal.owner_id).await?;
        if !has_history {
            return Err(Error::NotFound); // never been Pro; subscribe URL is not part of Free
        }
        // Webhook safety net: regenerate now and serve
        log::warn!(
            "frozen_subscribe_ics null for previously-paid user {} on calendar {}; lazy regenerating",
            cal.owner_id, cal.id
        );
        frozen_ics.regenerate(cal.id).await?;
        let refreshed = mapper.find_by_id(cal.id).await?
            .and_then(|c| c.frozen_subscribe_ics)
            .ok_or(Error::Internal("regenerate did not populate column".into()))?;
        Ok(HttpResponse::Ok()
            .content_type("text/calendar; charset=utf-8")
            .body(refreshed))
    }
}
```

- [x] Add `subscription_mapper.exists_for_user(user_id) -> bool`:

```rust
sqlx::query_scalar!(
    r#"SELECT EXISTS(SELECT 1 FROM subscriptions WHERE user_id = $1) AS "exists!""#,
    user_id
).fetch_one(&self.pool).await
```

- [x] Inline integration tests:

```rust
#[sqlx::test]
async fn subscribe_endpoint_pro_returns_live_data(pool: PgPool) { /* ... */ }

#[sqlx::test]
async fn subscribe_endpoint_free_with_blob_returns_blob(pool: PgPool) { /* ... */ }

#[sqlx::test]
async fn subscribe_endpoint_free_null_with_history_lazy_regen(pool: PgPool) {
    /* User has subscription history but column is null. Endpoint regenerates and returns blob. Column now populated. */
}

#[sqlx::test]
async fn subscribe_endpoint_free_no_history_returns_404(pool: PgPool) { /* ... */ }

#[sqlx::test]
async fn subscribe_token_unchanged_across_pro_free_pro(pool: PgPool) {
    /* Capture token at Pro creation, downgrade, capture again, upgrade, capture again — all equal. */
}
```

#### Stripe webhook hooks

- [x] In `server/src/controllers/stripe.rs`, after the existing subscription-mapper update:

```rust
// At the end of processing customer.subscription.deleted / .updated events,
// detect tier transitions and dispatch to FrozenIcsService.
let new_tier = entitlement.effective_tier(user_id).await?;
let old_tier = old_tier_before_update; // capture before subscription_mapper.update
match (old_tier, new_tier) {
    (Tier::Paid, Tier::Free) => {
        let failures = frozen_ics.regenerate_for_user(user_id).await?;
        if failures > 0 {
            log::warn!("frozen_ics: {failures} calendar(s) failed to regenerate for user {user_id}");
        }
    }
    (Tier::Free, Tier::Paid) => {
        frozen_ics.clear_for_user(user_id).await?;
    }
    _ => {}
}
```

- [x] Inline tests:

```rust
#[sqlx::test]
async fn webhook_downgrade_triggers_regen_for_all_user_calendars(pool: PgPool) {
    /* simulate customer.subscription.deleted; assert all of user's calendars now have non-null frozen_subscribe_ics */
}

#[sqlx::test]
async fn webhook_upgrade_nulls_blob_for_all_user_calendars(pool: PgPool) {
    /* simulate customer.subscription.created; assert all blobs null */
}

#[sqlx::test]
async fn webhook_anilist_failure_for_one_calendar_does_not_block_others(pool: PgPool) {
    /* mock data source that fails for one calendar; assert other calendars regenerated, returned warning logged */
}
```

#### Frontend — counters and gates

- [x] `frontend/src/services/calendars.ts`: add a method (or extend an existing one) that fetches the current show count + calendar count for the logged-in user. Could be a new `GET /api/account/usage` endpoint returning `{ shows: number, calendars: number }`, or piggyback on existing settings fetch — pick whichever fits the existing data-flow conventions (likely the latter).

- [x] `MyCalendarsPage.vue`:
  - For Free users, render a small chip near the page heading: `t('myCalendars.calendarCounter', { current: count, max: 3 })`.
  - Disable the "New calendar" button when `count >= 3`. Click while disabled opens the existing `UpgradeInterruptModal` with reason `cap_calendars`.
  - Hide chip and don't gate button for Pro users.
  - Use `data-testid="calendar-counter-chip"` and `data-testid="new-calendar-button"`.

- [x] `EditorItemsPanel.vue` (and the mobile variant):
  - For Free users, render a `12 / 25 shows` chip in the editor header. Use the show count derived from store / API.
  - At ≥ 80% of cap (default 20+), render a soft banner: *"Approaching your tracking limit (20 of 25). Upgrade to Pro for unlimited."* The banner is dismissible per-session.
  - At cap (25/25), disable add buttons in the search-result list when the result's media_id isn't already tracked (already-tracked items remain addable). Click on a disabled add button opens `UpgradeInterruptModal` with reason `cap_shows`.
  - Pro users see no chip, no banner, no disabled buttons.

- [x] Frontend axios layer / `services/calendars.ts`: 402 responses with `required_tier === "paid"` must trigger `UpgradeInterruptModal` with the appropriate reason code derived from a header or response body field. Add a `reason` field to the 402 response body server-side so the frontend doesn't have to infer:

```rust
// Update Error::PaymentRequired to include an optional reason
PaymentRequired { required_tier: &'static str, reason: Option<&'static str> }
```

Then at each call site: `Error::PaymentRequired { required_tier: "paid", reason: Some("cap_calendars") }` etc. Backwards compatible: existing accent enforcement passes `None`, frontend defaults to `pro_accent` reason.

- [x] Frontend tests:

```ts
// MyCalendarsPage.spec.ts
it('shows calendar counter for free user', async () => { /* ... */ });
it('hides counter for pro user', async () => { /* ... */ });
it('disables new calendar button at cap', async () => { /* ... */ });
it('opens upgrade modal with cap_calendars reason on disabled click', async () => { /* ... */ });

// EditorItemsPanel.spec.ts
it('shows show counter for free user', async () => { /* ... */ });
it('shows 80% warning banner at threshold', async () => { /* ... */ });
it('disables add buttons at cap for new media', async () => { /* ... */ });
it('keeps already-tracked add buttons enabled at cap', async () => { /* ... */ });
it('hides counter and banner for pro user', async () => { /* ... */ });

// api.spec.ts
it('intercepts 402 with required_tier=paid and reason=cap_shows', async () => {
    /* mock 402 → assert UpgradeInterruptModal opens with reason="cap_shows" */
});
```

Apply memory `feedback_account_tab_router_stub_coupling` if changes to upgrade-modal trigger logic touch shared infrastructure.

### Acceptance

- All Phase 0 acceptance still holds.
- `cargo test --workspace` — green including new entitlement, show-count, frozen-ics, controller integration tests.
- `cd frontend && npm run test:unit` — green including new counter / banner / modal tests.
- Manual: sign in as a Free user, create 3 calendars, attempt 4th → modal opens. Add 25 shows, attempt 26th new one → modal opens. Try to add a show that's already tracked → succeeds.
- Manual: subscribe URL on Pro user → live `.ics`. Downgrade via `set_subscription` CLI. Subscribe URL → `.ics` from frozen blob. Re-upgrade. Subscribe URL → live again.
- Manual: brand-new free user (never been Pro) — guess their subscribe token URL via DB query, hit it → 404.

### Suggested commits

1. `feat(db): add event_style + frozen_subscribe_ics + reminder_offsets columns`
2. `feat(server): show count service + entitlement assert helpers`
3. `feat(server): frozen ics service + ics_export refactor scaffold`
4. `feat(server): tier-aware subscribe endpoint with history-check fallback`
5. `feat(server): stripe webhook frozen-blob lifecycle hooks`
6. `feat(server): per-user advisory lock around calendar create + items add`
7. `feat(frontend): calendar + show counters with cap warnings and modal triggers`
8. `feat(server): structured 402 reason codes; frontend modal reason routing`

---

## Phase 2 — Composable reminders + per-calendar event style

**Goal:** the `ics_export` service that landed as a scaffold in Phase 1 becomes feature-complete: emits one `VALARM` per stored offset for Pro, always single 30-min for Free. Calendars switch between `timed` and `all_day` modes via the settings panel. Reminders chip-list ships in Account → Preferences with the 5-cap visible-from-the-start UX.

### Tasks

#### Backend — ics_export feature work

- [x] `IcsExportService::render` flesh-out:
  - Load calendar (incl. `event_style`) + items + owner's `user_settings.reminder_offsets_minutes`.
  - For each item, resolve via `CachedDataSource::fetch_by_ids` (single batched call for all items).
  - For each item / episode tuple:
    - If `calendar.event_style == "timed"` AND item has a known `airing_at`: emit `DTSTART:datetime` + `DTEND:datetime` (or `DURATION:PT24M` per existing convention).
    - Else (event_style is `all_day` OR `airing_at` is None): emit `DTSTART;VALUE=DATE` for the airing date in UTC.
  - For each event:
    - If owner is **Pro**: take `reminder_offsets_minutes` (capped at first 5 for safety), emit one `VALARM` per offset:
      ```
      BEGIN:VALARM
      TRIGGER:-PT<minutes>M  (or formatted as -P<days>D for ≥ 1440)
      ACTION:DISPLAY
      DESCRIPTION:<event summary>
      END:VALARM
      ```
    - If owner is **Free**: emit exactly one `VALARM` with `TRIGGER:-PT30M`, regardless of stored offsets. **Critical anti-bypass: this must be enforced server-side, not by trusting the stored array.**

- [x] Tests in `ics_export.rs`:

```rust
#[sqlx::test]
async fn pro_user_with_three_offsets_emits_three_valarms(pool: PgPool) {
    let user = make_pro_user_with_offsets(&pool, vec![30, 60, 1440]).await;
    let cal = make_calendar_with_one_event(&pool, user.id).await;
    let blob = service.render(cal.id).await.unwrap();
    let valarm_count = blob.matches("BEGIN:VALARM").count();
    assert_eq!(valarm_count, 3);
    assert!(blob.contains("TRIGGER:-PT30M"));
    assert!(blob.contains("TRIGGER:-PT1H"));   // 60 minutes — formatter's choice
    assert!(blob.contains("TRIGGER:-P1D"));    // 1440 minutes
}

#[sqlx::test]
async fn pro_user_with_zero_offsets_emits_zero_valarms(pool: PgPool) {
    let user = make_pro_user_with_offsets(&pool, vec![]).await;
    let cal = make_calendar_with_one_event(&pool, user.id).await;
    let blob = service.render(cal.id).await.unwrap();
    assert!(!blob.contains("BEGIN:VALARM"));
}

#[sqlx::test]
async fn free_user_always_emits_single_30min_valarm_regardless_of_stored_offsets(pool: PgPool) {
    // Stored offsets simulate a former Pro user who downgraded.
    let user = make_free_user_with_stored_offsets(&pool, vec![60, 1440]).await;
    let cal = make_calendar_with_one_event(&pool, user.id).await;
    let blob = service.render(cal.id).await.unwrap();
    let valarm_count = blob.matches("BEGIN:VALARM").count();
    assert_eq!(valarm_count, 1);
    assert!(blob.contains("TRIGGER:-PT30M"));
    assert!(!blob.contains("TRIGGER:-PT1H"));
    assert!(!blob.contains("TRIGGER:-P1D"));
}

#[sqlx::test]
async fn event_style_timed_with_air_time_emits_dtstart_with_time(pool: PgPool) { /* ... */ }

#[sqlx::test]
async fn event_style_timed_without_air_time_falls_back_to_value_date(pool: PgPool) {
    // Calendar event_style=timed but item has no airing_at → DTSTART;VALUE=DATE
}

#[sqlx::test]
async fn event_style_all_day_always_emits_value_date(pool: PgPool) {
    // Even when airing_at is known, all_day mode emits date-only
}
```

#### Backend — settings validation

- [x] In `server/src/controllers/user.rs`, the `update_user_settings` handler. Add validation for `reminder_offsets_minutes`:

```rust
const CANONICAL_REMINDER_OFFSETS: &[i32] = &[15, 30, 60, 120, 360, 720, 1440, 2880, 4320, 10080];

fn validate_reminder_offsets(offsets: &[i32]) -> Result<(), &'static str> {
    if offsets.len() > 5 {
        return Err("reminder_offsets_invalid");
    }
    for &offset in offsets {
        if !CANONICAL_REMINDER_OFFSETS.contains(&offset) {
            return Err("reminder_offsets_invalid");
        }
    }
    Ok(())
}
```

- [x] Apply at the top of the handler, before any DB write:

```rust
if let Err(code) = validate_reminder_offsets(&payload.reminder_offsets_minutes) {
    return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": code})));
}
```

- [x] **Critically** — do **not** reject a Free user setting non-default reminders. Per the spec, those values are stored for the eventual upgrade and ignored at .ics emission. The 200-stored-but-ignored behaviour is part of the tier-transition continuity story (memory `feedback_tier_gated_apply_pattern`).

- [x] Tests:

```rust
#[sqlx::test]
async fn update_settings_rejects_six_reminders(pool: PgPool) { /* ... */ }

#[sqlx::test]
async fn update_settings_rejects_uncanonical_offset(pool: PgPool) { /* offset = 17 → 400 */ }

#[sqlx::test]
async fn update_settings_accepts_empty_array(pool: PgPool) { /* offsets = [] → 200 */ }

#[sqlx::test]
async fn update_settings_free_user_can_store_pro_offsets(pool: PgPool) {
    /* Free user PUT with [60, 1440] → 200, row updated */
}

#[sqlx::test]
async fn update_settings_preserves_offsets_across_pro_to_free(pool: PgPool) {
    /* Pro user sets [60, 1440]. Downgrade. Settings row still has [60, 1440]. */
}
```

#### Backend — calendar event_style validation

- [x] In the `PUT /calendar` payload deserialization, validate `event_style` against `{"timed", "all_day"}`. Reject with 400 + `{"error":"event_style_invalid"}` for anything else. Preserve in `serde` if using `#[serde(rename_all)]` enum — `#[derive(Deserialize)] enum EventStyle { Timed, AllDay }` with proper rename works fine.

- [x] Test: `put_calendar_invalid_event_style_returns_400`.

#### Frontend — Reminders section in PreferencesTab

- [ ] In `account/PreferencesTab.vue`, add a new "Reminders" section. Render a chip list of the 10 canonical offsets:

```vue
<section class="space-y-3">
  <header class="flex items-center gap-2">
    <h3 class="font-display text-xl text-fg-1">{{ t('userSettings.reminders.heading') }}</h3>
    <span class="text-sm text-fg-3">{{ t('userSettings.reminders.cap', { active: activeCount, max: 5 }) }}</span>
    <button
      type="button"
      class="text-fg-3"
      :title="t('userSettings.reminders.capExplanation')"
      data-testid="reminders-info-icon"
      @click.prevent
    >
      <IconInfo />
    </button>
  </header>

  <div v-if="!isPaid" class="rounded-md border border-line bg-bg-2 p-4">
    <p class="text-sm text-fg-2">{{ t('userSettings.reminders.proLockedMessage') }}</p>
    <UiButton variant="primary" size="sm" class="mt-2" @click="goToUpgrade">
      {{ t('userSettings.reminders.upgradeCta') }}
    </UiButton>
  </div>

  <ul class="flex flex-wrap gap-2" :aria-disabled="!isPaid" data-testid="reminders-chip-list">
    <li v-for="offset in CANONICAL_REMINDER_OFFSETS" :key="offset">
      <button
        type="button"
        :class="chipClasses(offset)"
        :disabled="!isPaid || (atCap && !isActive(offset))"
        :aria-pressed="isActive(offset)"
        :data-testid="`reminder-chip-${offset}`"
        @click="toggleOffset(offset)"
      >
        {{ formatOffset(offset) }}
      </button>
    </li>
  </ul>
</section>
```

- [ ] Logic in script setup:

```ts
const CANONICAL_REMINDER_OFFSETS = [15, 30, 60, 120, 360, 720, 1440, 2880, 4320, 10080] as const

const settingsStore = useUserSettingsStore()
const activeOffsets = computed(() => settingsStore.reminderOffsetsMinutes ?? [])
const activeCount = computed(() => activeOffsets.value.length)
const atCap = computed(() => activeCount.value >= 5)
const isActive = (offset: number) => activeOffsets.value.includes(offset)

function toggleOffset(offset: number) {
  if (!isPaid.value) return
  const current = [...activeOffsets.value]
  const idx = current.indexOf(offset)
  if (idx >= 0) {
    current.splice(idx, 1)
  } else if (current.length >= 5) {
    return // disabled-when-full pattern; no auto-deselect
  } else {
    current.push(offset)
    current.sort((a, b) => a - b)
  }
  settingsStore.updateReminderOffsets(current)
}

function formatOffset(minutes: number): string {
  if (minutes < 60) return t('userSettings.reminders.offset.minutes', { n: minutes })
  if (minutes < 1440) return t('userSettings.reminders.offset.hours', { n: minutes / 60 })
  if (minutes < 10080) return t('userSettings.reminders.offset.days', { n: minutes / 1440 })
  return t('userSettings.reminders.offset.weeks', { n: minutes / 10080 })
}
```

- [ ] In `userSettingsStore.ts`: add `reminderOffsetsMinutes: number[]`, `updateReminderOffsets(offsets: number[])` action that calls the existing settings PUT endpoint with the new shape.

- [ ] Tests:

```ts
// PreferencesTab.spec.ts
it('renders 10 reminder chips', async () => { /* ... */ });
it('pro user can toggle a chip active', async () => { /* ... */ });
it('pro user with 5 active sees inactive chips disabled', async () => {
    // Mount with store state showing 5 active offsets.
    // For each inactive offset, expect chip to have disabled attr.
});
it('clicking disabled chip at cap is no-op', async () => {
    // Mount with 5 active. Click a disabled chip. Expect store update NOT called.
});
it('info icon hover (or click) shows cap explanation', async () => { /* ... */ });
it('free user sees pro-locked overlay with upgrade cta', async () => { /* ... */ });
it('clicking upgrade cta navigates to /upgrade', async () => { /* ... */ });
```

Apply memory `feedback_account_tab_router_stub_coupling` — add the new tab content but verify `AccountPage.spec.ts` router stub still covers the route shape.

#### Frontend — event_style toggle in CalendarSettingsForm

- [ ] In `CalendarSettingsForm.vue`, add a `UiSegmented` toggle:

```vue
<div class="space-y-2">
  <label class="font-medium text-fg-1">{{ t('calendar.settings.eventStyle.label') }}</label>
  <UiSegmented
    v-model="form.eventStyle"
    :options="eventStyleOptions"
    :aria-label="t('calendar.settings.eventStyle.ariaLabel')"
    data-testid="event-style-segmented"
  />
  <p class="text-sm text-fg-3">
    {{ t('calendar.settings.eventStyle.fallbackNote') }}
  </p>
</div>
```

- [ ] Options in script setup:

```ts
const eventStyleOptions = computed(() => [
  { value: 'timed', label: t('calendar.settings.eventStyle.timedLabel'), description: t('calendar.settings.eventStyle.timedDescription') },
  { value: 'all_day', label: t('calendar.settings.eventStyle.allDayLabel'), description: t('calendar.settings.eventStyle.allDayDescription') },
])
```

- [ ] The fallback note copy MUST include the all-day-on-missing-air-time explanation explicitly. Example en.json copy: *"Episodes without a known air time will appear as all-day events even in Timed mode."*

- [ ] Form submission: `event_style` passes through to the existing PUT /calendar payload.

- [ ] Tests:

```ts
// CalendarSettingsForm.spec.ts
it('event style segmented renders both options', async () => { /* ... */ });
it('selecting timed persists to form model', async () => { /* ... */ });
it('selecting all_day persists to form model', async () => { /* ... */ });
it('fallback note mentions all-day fallback', async () => {
    // Mount, find the note element by data-testid, assert it contains
    // text matching the i18n key. Critical copy-presence test.
});
```

#### Locales

- [ ] Add to both `en.json` and `pt.json`:

```jsonc
{
  "userSettings": {
    "reminders": {
      "heading": "Reminders",
      "cap": "{active} of {max} active",
      "capExplanation": "Calendar apps may batch or ignore notifications beyond five per event. We cap it here to keep alerts useful.",
      "proLockedMessage": "Customise reminders with Anime Calendar Pro.",
      "upgradeCta": "Upgrade to Pro",
      "offset": {
        "minutes": "{n} min",
        "hours": "{n} hour | {n} hours",
        "days": "{n} day | {n} days",
        "weeks": "{n} week | {n} weeks"
      }
    }
  },
  "calendar": {
    "settings": {
      "eventStyle": {
        "label": "Event style",
        "ariaLabel": "Choose between timed and all-day events",
        "timedLabel": "Timed",
        "timedDescription": "Show precise air times.",
        "allDayLabel": "Daily",
        "allDayDescription": "All-day events for a roundup view.",
        "fallbackNote": "Episodes without a known air time will appear as all-day events even in Timed mode."
      }
    }
  }
}
```

- [ ] Run the i18n contract test (or manual `npm run lint:i18n` if it exists) — keys must parity-match across locales.

### Acceptance

- All Phase 0–1 acceptance still holds.
- Manual: as Pro user, select 3 reminder offsets → download .ics → open in a text editor → confirm three `VALARM` blocks with correct `TRIGGER` values.
- Manual: as Free user with 3 stored offsets from a prior Pro period → download .ics → confirm exactly one 30-min `VALARM`.
- Manual: switch a calendar to all-day mode → download .ics → confirm `DTSTART;VALUE=DATE` for each event regardless of known air times.
- Manual: switch a calendar back to timed mode → confirm `DTSTART:datetime` for events with air times, `DTSTART;VALUE=DATE` for events without.
- Manual: as Free user, attempt to set 6 reminders via dev-tools → server returns 400.
- Frontend tests cover chip cap behaviour without auto-deselect.

### Suggested commits

1. `feat(server): ics_export emits per-event valarm list with tier-aware filter`
2. `feat(server): event_style branching + missing-air-time fallback in ics_export`
3. `feat(server): validate reminder_offsets_minutes payload (1-5, canonical set)`
4. `feat(frontend): reminders chip-list section in PreferencesTab with pro-lock overlay`
5. `feat(frontend): event style segmented toggle in CalendarSettingsForm`
6. `chore(i18n): reminders + event-style locale keys (en + pt)`

---

## Phase 3 — Pricing reset + UpgradePage rewrite + locale cleanup

**Goal:** ship the new $2.99/$24.99 pricing, refresh the UpgradePage to advertise only what's actually built (no AniList sync, no sharing — those land in their own specs), retire dead marketing copy from the locales, and run the Stripe-side price update via the companion checklist.

### Tasks

#### Backend — config + Stripe price IDs

- [ ] Update `config.toml.dist` with placeholders for the new prices:

```toml
[stripe.prices]
monthly = "price_REPLACE_ME_MONTHLY"
annual  = "price_REPLACE_ME_ANNUAL"
```

- [ ] Update `config.toml` (gitignored) with the new test-mode `price_*` IDs after creating them in Stripe Dashboard per `docs/checklists/2026-05-monetisation-stripe-price-update.md` §1.
- [ ] No code changes — the existing checkout flow already uses `config.stripe.prices.monthly` / `.annual`.

#### Frontend — UpgradePage rewrite

- [ ] Rewrite the Pro feature list constant in `UpgradePage.vue`. Replace the existing `PRO_FEATURE_KEYS` array:

```ts
const PRO_FEATURE_KEYS = [
  'pricing.features.unlimitedCalendars',
  'pricing.features.unlimitedShows',
  'pricing.features.liveSubscribeUrl',
  'pricing.features.customisableReminders',
  'pricing.features.allAccents',
  'pricing.features.earlyAccess',
] as const
```

- [ ] Update the price strings via locale keys (no hardcoded prices in component). Existing `pricing.price.monthly`, `pricing.price.annual`, `pricing.price.savings`, and `pricing.price.perMonth` / `perYear` already exist — just update the values in the locale files.
- [ ] No layout / structural changes — the existing two-column hero pattern is preserved.

- [ ] Tests:

```ts
// UpgradePage.spec.ts
it('renders the new pricing strings', async () => {
    const wrapper = mount(UpgradePage, /* ... */);
    expect(wrapper.text()).toContain('$2.99');
    expect(wrapper.text()).toContain('$24.99');
});

it('renders six pro features', async () => {
    const wrapper = mount(UpgradePage, /* ... */);
    const features = wrapper.findAll('[data-testid="pro-feature"]');
    expect(features.length).toBe(6);
});

it('does not advertise unbuilt features', async () => {
    const wrapper = mount(UpgradePage, /* ... */);
    const text = wrapper.text();
    expect(text).not.toContain('AniList sync');
    expect(text).not.toContain('MyAnimeList');
    expect(text).not.toContain('shared editors');
    expect(text).not.toContain('Studio');
});
```

If `pro-feature` data-testids aren't on each feature `<li>`, add them as part of this work.

#### Locale cleanup

- [ ] In `en.json`, update price-related keys:

```jsonc
"pricing": {
  "price": {
    "monthly": "$2.99",
    "annual": "$24.99",
    "perMonth": "per month",
    "perYear": "per year",
    "savings": "Save ~30%"
  }
}
```

Also update the same keys in `pt.json` (translated). Keep the same numeric values across locales for v1 (USD-only billing).

- [ ] Add new feature keys (in both locales):

```jsonc
"pricing": {
  "features": {
    "unlimitedCalendars": "Unlimited calendars",
    "unlimitedShows": "Unlimited tracked shows",
    "liveSubscribeUrl": "Live subscribe URL with hourly refresh",
    "customisableReminders": "Customisable reminders (up to 5)",
    "allAccents": "All accent themes — Coral, Iris, Matcha, Sakura, Citron",
    "earlyAccess": "Early access to new features"
  }
}
```

(Translate values in `pt.json` accordingly.)

- [ ] **Remove** retired keys from both locales:
  - `pricing.features.proAccents` (replaced by `allAccents`)
  - `pricing.features.priorityRefresh` (retired)
  - `pricing.features.exportFlexibility` (retired)
  - `pricing.features.support` (retired)
  - `pricing.tiers.free.features.noPrioritySupport` (retired)
  - Any other `pricing.features.*` that the new feature list doesn't reference.
- [ ] Update `pricing.tiers.free.features.*`:

```jsonc
"free": {
  "features": {
    "tracking": "Anime tracking and weekly schedule",
    "limitedCalendars": "Up to 3 calendars",
    "limitedShows": "Up to 25 tracked shows",
    "icsExport": ".ics download export",
    "freeAccents": "Coral and Iris accents",
    "noProAccents": "Pro accent themes (Matcha, Sakura, Citron)",
    "noLiveSubscribe": "Live subscribe URL with hourly refresh",
    "noCustomReminders": "Customisable reminders"
  }
}
```

Then update the `FREE_FEATURES` array in `UpgradePage.vue` to reference the new keys:

```ts
const FREE_FEATURES: readonly FeatureRow[] = [
  { ok: true,  textKey: 'pricing.tiers.free.features.tracking' },
  { ok: true,  textKey: 'pricing.tiers.free.features.limitedCalendars' },
  { ok: true,  textKey: 'pricing.tiers.free.features.limitedShows' },
  { ok: true,  textKey: 'pricing.tiers.free.features.icsExport' },
  { ok: true,  textKey: 'pricing.tiers.free.features.freeAccents' },
  { ok: false, textKey: 'pricing.tiers.free.features.noProAccents' },
  { ok: false, textKey: 'pricing.tiers.free.features.noLiveSubscribe' },
  { ok: false, textKey: 'pricing.tiers.free.features.noCustomReminders' },
] as const
```

- [ ] Run the locales parity gate (vitest contract test or `npm run lint:i18n`) — should pass.
- [ ] Search the codebase for any references to removed keys — `grep -r "priorityRefresh\|exportFlexibility\|pricing.features.support" frontend/src/` should return nothing after cleanup.

#### Stripe rollout

- [ ] Run `docs/checklists/2026-05-monetisation-stripe-price-update.md` §1–§4 in test mode.
- [ ] After test-mode validation passes, defer §5–§6 (live mode) until ready to deploy. Live-mode price creation can be a separate PR; spec 1 doesn't require live billing.

### Acceptance

- Frontend build clean, all tests green.
- Manual: open `/upgrade` → confirm prices read $2.99 / $24.99, six Pro features listed, no AniList / sharing / Studio text anywhere on the page.
- Manual: click "Start free trial" → Stripe Checkout opens → confirm line item shows $2.99 with 14-day trial banner. (Per checklist §2.)
- Manual: switch interval to Annual → Checkout shows $24.99 line item.
- `grep` for retired locale keys returns zero hits.
- `cd frontend && npm run lint && npm run test:unit` green.

### Suggested commits

1. `feat(frontend): rewrite UpgradePage feature list to match v1 enforcement`
2. `chore(i18n): pricing reset to $2.99 / $24.99; retire dead marketing keys`
3. `chore(stripe): switch test-mode price IDs in config.toml`

---

## Cross-phase wrap-up

After Phase 3 lands, before declaring spec 1 done:

- [ ] Update `MEMORY.md` index with a new `project_monetisation_v1_complete.md` entry summarising what shipped and any non-obvious gotchas surfaced during implementation.
- [ ] Run the full Track 4 readiness checklist (`docs/checklists/2026-05-track-4-release-readiness.md`) end-to-end — Stripe webhook + reconcile loop + portal interactions should all still work; nothing in this spec touches that infrastructure.
- [ ] Run the new `docs/checklists/2026-05-monetisation-stripe-price-update.md` §1–§4 (test mode) to confirm the price-side of the rollout is clean.
- [ ] Append a small additional manual checklist (or extend the new one) covering: cap reach + downgrade-and-poll subscribe URL + reminder customisation + event-style toggle. Live with the spec or as a third checklist file at the runner's discretion.
- [ ] Confirm the data-source spike memo (`docs/superpowers/specs/2026-05-04-data-source-spike.md`, if created during this period) is filed. If the spike resolved on a non-AniList source, queue the migration spec.
- [ ] Sibling specs ready for follow-up brainstorm:
  - Spec 2: Co-editor sharing
  - Spec 3: AniList OAuth + list sync
  - Spec 4: MAL OAuth + list sync

## Notes for the implementer

- **Test memory gotchas to apply throughout:**
  - `feedback_uimodal_teleport_tests` — modal-touching frontend tests need `attachTo: document.body` and query `document.body`.
  - `feedback_test_history_pollution` — `createMemoryHistory` only.
  - `feedback_account_tab_router_stub_coupling` — touching `PreferencesTab` may require `AccountPage.spec.ts` router stub updates.
  - `feedback_smoke_test_timer_leak` — fake timers or trigger error branch on `onMounted` `setTimeout(router.push, ...)`.
  - `feedback_jsdom_focusable_offsetparent` — don't filter focus traps by `offsetParent !== null` in Teleport-using components.
  - `feedback_axe_oklch_false_positives` — re-verify any axe-flagged contrast issues with canvas / alpha-composite approach.
  - `feedback_sqlx_offline_cache` — regenerate + commit `.sqlx/` after every sqlx query change with `--all-targets`.

- **Memory note `feedback_in_tx_static_helper_pattern`** is relevant for the Phase 1 transactional locking — if other mappers need to participate in the same transaction (e.g., `frozen_ics.regenerate` needs to read calendar state right after insert), expose `_in_tx(conn, ...)` helpers rather than passing pools around.

- **Don't pre-advertise sibling-spec features.** The temptation is to add "AniList sync — coming soon" to UpgradePage. Resist; the page should only show shipped features. When spec 3 ships, that PR adds the AniList sync entry.
