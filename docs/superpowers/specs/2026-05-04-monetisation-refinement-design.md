# Monetisation Refinement — Tier Gates, Caching, Pricing Reset

**Date:** 2026-05-04
**Status:** Spec / brainstormed (pending implementation plan)
**Predecessor:** Track 4 (Pro tier + Stripe) ✅
**Companion checklists:**
- `docs/checklists/2026-05-track-4-release-readiness.md` (existing)
- `docs/checklists/2026-05-monetisation-stripe-price-update.md` (new — runs the price change)

**Sibling specs (deferred, separate documents):**
1. **Spec 2:** Co-editor sharing (calendar collaborators, magic-link invites).
2. **Spec 3:** AniList OAuth login + AniList list sync.
3. **Spec 4:** MyAnimeList OAuth login + MAL list sync.
4. **Pre-spec memo:** Data-source spike (AnimeSchedule.net + Kitsu evaluation), runs in parallel with Phase 0 of this spec.

## Goals

1. Replace the marketing-only Pro feature list with a coherent, enforced tier model that recovers hosting + AniList API costs while maximising free-to-paid conversion.
2. Lower pricing to the impulse-purchase tier ($2.99/mo, $24.99/yr — ~30% off) appropriate for a single-purpose anime companion utility.
3. Build a foundation that lets the data source be swapped (AniList → AnimeSchedule.net / Kitsu / hybrid) without rewriting downstream code.
4. Improve cache efficiency by caching individual items across users (not just request batches), splitting metadata vs airing TTLs, and moving all TTLs into config.
5. Make all tier gates honest: every Pro feature listed on the upgrade page is actually enforced server-side, and every retired marketing claim is removed from the locales.

## Non-goals (explicit)

- **Co-editor sharing** — separate spec.
- **AniList / MyAnimeList list sync + OAuth login** — separate specs.
- **Read-only public web view** of calendars (subscribe URL stays the only sharing mechanism for v1).
- **Lifetime tier / one-time purchase** — recurring monthly + annual only.
- **Multi-currency** — USD only.
- **Per-show reminder overrides** — account-wide reminder list only. Per-show is a v2 enhancement.
- **Cache key versioning** — pre-release; breaking cache changes are free. Add versioned prefix once deployed.
- **Stripe Customer Portal redesign** — existing flow from Track 4 is reused.
- **Grandfathering for existing paid users** — pre-release, no paid users exist yet.

## Locked-in product decisions

| Topic | Decision |
|---|---|
| Tiers | Free + Pro (binary, unchanged from Track 4 schema) |
| Pricing | **$2.99/mo · $24.99/yr** (~30% off vs 12 × monthly) |
| Trial | 14 days (preserved from Track 4) |
| Free calendar limit | 3 |
| Free show cap | **25 distinct media IDs** across the whole account |
| Free export | `.ics` download only (subscribe URL serves a frozen blob) |
| Free reminders | **Single hardcoded 30-min `VALARM`**, no UI exposed |
| Free accents | Coral + Iris (unchanged) |
| Pro calendars | Unlimited |
| Pro shows | Unlimited |
| Pro export | Live subscribe URL, hourly refresh cadence |
| Pro reminders | Composable list of up to **5** reminders chosen from 10 canonical offsets |
| Pro accents | Matcha + Sakura + Citron (unchanged) |
| Per-calendar event style | `timed` or `all_day`, available to **both** tiers, falls back to all-day when AniList lacks an air time |
| Downgrade behaviour | **Soft freeze**: calendars and shows persist; new adds blocked while over cap; .ics frozen |
| Subscribe-token lifecycle | **Never rotates** across Pro→Free→Pro |
| Subscribe URL for never-Pro user | **404** (anti-enumeration; feature is not part of Free) |

### Reminder canonical offsets (Pro)

`{15m, 30m, 1h, 2h, 6h, 12h, 1d, 2d, 3d, 1w}` — stored as integer minutes: `{15, 30, 60, 120, 360, 720, 1440, 2880, 4320, 10080}`. Maximum 5 active. The default for a new Pro user is `[30]` (matches the Free baseline so transitions feel continuous).

## Architecture

### Modules touched

**Backend (Rust):**

| Module | Change |
|---|---|
| `anilist/src/lib.rs` | Define `AnimeDataSource` trait. Existing AniList code becomes the first impl. |
| `server/src/services/cached_data_source.rs` (new) | Generic `CachedDataSource<S: AnimeDataSource>` adapter. Per-item check → batched upstream → split-TTL writeback. |
| `server/src/cache.rs` | Add `generate_item_meta_key`, `generate_item_airing_key`. Drop `generate_items_key` (redundant). Read TTLs from config. |
| `server/src/services/entitlement.rs` | Add `assert_can_create_calendar`, `assert_can_add_show`. |
| `server/src/services/show_count.rs` (new) | `count_distinct_shows_for_user(user_id)` over `calendar_items × calendars`. |
| `server/src/services/frozen_ics.rs` (new) | `regenerate(calendar_id)`, `regenerate_for_user(user_id)`, `clear_for_user(user_id)`. |
| `server/src/services/ics_export.rs` (refactor of inline rendering) | Branches on `event_style`; emits one `VALARM` per stored offset; falls back to all-day for missing air times. |
| `server/src/controllers/calendar.rs` | Inject entitlement checks into `PUT /calendar` (create branch) and item-add. Subscribe endpoint branches on owner tier. |
| `server/src/controllers/stripe.rs` | Webhook handler triggers frozen-blob regeneration on Pro→Free, clears it on Free→Pro. |
| `server/src/entity/calendar.rs` + migration | Add `event_style`, `frozen_subscribe_ics` columns. |
| `server/src/entity/user_settings.rs` + migration | Add `reminder_offsets_minutes` integer array. |
| `config.toml` + config struct | New `[cache]` and `[limits]` blocks. |

**Frontend (Vue):**

| Component | Change |
|---|---|
| `UpgradePage.vue` | New pricing copy, six-feature Pro list, no pre-advertised AniList sync / sharing. |
| `MyCalendarsPage.vue` | `2 / 3 calendars` chip (Free), disabled "New" button at cap with upgrade modal. |
| `calendar/EditorItemsPanel.vue` (+ mobile variant) | `12 / 25 shows` chip (Free), 80% banner, hard-block at 100%. |
| `account/PreferencesTab.vue` | Reminders chip-list section; Pro-locked overlay for Free. |
| `CalendarSettingsForm.vue` | Event-style `UiSegmented` toggle with all-day-fallback explanation in tooltip. |
| `stores/userSettingsStore.ts` | Track `reminder_offsets_minutes`. |
| `services/calendars.ts`, `services/userSettingsService.ts` | Extend payload shapes. |
| `locales/en.json`, `locales/pt.json` | Add new keys (caps, reminders, event style, upgrade page rewrite); remove `pricing.features.priorityRefresh`, `pricing.features.exportFlexibility`, `pricing.features.support`. |

**External:** Stripe Dashboard — new monthly + annual prices on the existing Pro product. See `docs/checklists/2026-05-monetisation-stripe-price-update.md`.

### Boundaries

- **`AnimeDataSource` trait is the only contract for upstream data.** Everything above speaks `common::Item`; impls translate from native shapes. The data-source spike's outcome becomes a swap-in change later, not a rewrite.
- **`EntitlementService` is the only place tier rules live.** Controllers ask "can this user do X?" — they never branch on `Tier::Free`/`Tier::Paid`.
- **`FrozenIcsService` is the only writer to `frozen_subscribe_ics`.** Subscribe controller is the only reader. Read/write asymmetry keeps the invariant clear.
- **TTLs and limits read from config exactly once at startup**, injected via `web::Data<Config>`. No hot-reload in v1.

### Configuration additions

```toml
[cache]
metadata_ttl_seconds       = 86400   # 24h — static AniList fields
airing_ttl_seconds         = 900     # 15m — airing schedule
search_ttl_seconds         = 3600    # 1h
calendar_items_ttl_seconds = 300     # 5m
export_ttl_seconds         = 300     # 5m

[limits]
free_calendar_limit = 3
free_show_cap       = 25
pro_max_reminders   = 5
```

## Phasing (Approach A — foundation-first)

### Phase 0 — Caching foundation + trait extraction

User-invisible. Backwards-compatible refactor.

- Define `AnimeDataSource` trait in `anilist` crate.
- Implement `CachedDataSource<S>` adapter with split metadata/airing TTLs.
- Update `cache.rs` keys (`item:meta:{id}`, `item:airing:{id}`); drop `generate_items_key`.
- Move all TTLs to `config.toml`.
- Wire dependency injection: controllers consume `web::Data<dyn AnimeDataSource>` (or a typed alias) instead of the concrete client.

### Phase 1 — Tier infrastructure

- DB migration: `calendars.event_style`, `calendars.frozen_subscribe_ics`, `user_settings.reminder_offsets_minutes`.
- `EntitlementService` extensions (`assert_can_create_calendar`, `assert_can_add_show`).
- `ShowCountService` (count distinct media IDs).
- `FrozenIcsService` (regenerate / regenerate_for_user / clear_for_user).
- Wire entitlement checks into the `PUT /calendar` handler at two points: (a) the calendar-creation branch (`id.is_none()` in the payload) and (b) the items-diff branch (every newly-introduced `media_id` in the request relative to the persisted calendar). Use a per-user transactional lock around the count → insert critical section.
- Subscribe endpoint branches on owner tier per "Flow B" below.
- Stripe webhook handler triggers `regenerate_for_user` on Pro→Free, `clear_for_user` on Free→Pro.
- Frontend: items-cap counter + warnings + hard-block in calendar editor; `2 / 3 calendars` chip + disabled-button on `MyCalendarsPage`. 402 responses route to existing `UpgradeInterruptModal` with new `cap_calendars` / `cap_shows` reason codes.

### Phase 2 — Reminders + per-calendar event style

- Refactor `.ics` rendering into a centralised `ics_export` service. Branch on `event_style`. Emit one `VALARM` per stored offset. Free users always emit a single 30-min `VALARM` regardless of stored offsets (defense-in-depth).
- Validate `reminder_offsets_minutes` payload server-side: array of 1–5 integers, each in the canonical set.
- Frontend: reminders chip-list section in `PreferencesTab`; Pro-locked overlay for Free with upgrade CTA. Event-style `UiSegmented` in `CalendarSettingsForm` with tooltip copy that mentions the all-day fallback.

### Phase 3 — Pricing + UpgradePage rewrite + locale cleanup

- `UpgradePage.vue` rewrite: $2.99 / $24.99 prices, six Pro features (no pre-advertised AniList sync or sharing).
- Locale cleanup: add new keys, remove retired ones.
- Stripe price IDs in `config.toml` updated to the new test-mode prices created via the companion checklist.

## Data flow

### Flow A — Free user adds a show

```
[User clicks Add on a search result]
        │
        ▼
[Frontend pre-flight]
   isPro?                                        ──→ proceed
   Free + shows_tracked < cap                    ──→ proceed
   Free + shows_tracked >= cap +
       media_id already tracked elsewhere        ──→ proceed (idempotent)
   Free + shows_tracked >= cap +
       new media_id                              ──→ open UpgradeInterruptModal
                                                       (cap_shows reason)
        │
        ▼ (if allowed)
[Calendar mutation request to the existing PUT /calendar handler,
 with the full calendar payload including the new item]
        │
        ▼
[Server: entitlement.assert_can_add_show(user_id, media_id)
                            for each newly-introduced media_id in the diff]
   Free + new media_id + count >= cap  → 402 PaymentRequired
   Free + already-tracked media_id     → Ok (idempotent — count unchanged)
   Free + count < cap                  → Ok
   Pro                                 → Ok
        │
        ▼
[Transactional UPSERT of calendar + items inside per-user lock]
        │
        ▼
[Invalidate calendar:items:{id}, export:{id};
 if owner is Free, frozen_ics.regenerate(calendar_id)]
        │
        ▼
[Return updated calendar payload]
```

> **Endpoint-shape note.** The existing `PUT /calendar` handler (`server/src/controllers/calendar.rs:374`) accepts a full calendar payload for both create and update. Item-add semantics fall out of diffing the payload's items array against the persisted state. The plan writeup will confirm whether the entitlement check lives directly in the handler or in a service helper invoked from the items-diff branch — either is acceptable as long as the invariant holds.

### Flow B — Subscribe URL poll (revised, with tier-history check)

```
[GET /api/calendars/subscribe/{token}]
        │
        ▼
[SELECT id, owner_id, frozen_subscribe_ics, event_style FROM calendars WHERE subscribe_token = $1]
   row not found → 404
        │
        ▼
[entitlement.effective_tier(owner_id)]
        │
        ├── Tier::Paid ──→ live path (cached items via CachedDataSource)
        │                   ↓
        │                   200 OK + .ics body
        │
        └── Tier::Free
                ↓
         frozen_subscribe_ics IS NOT NULL?
                ├── yes ──→ return blob (common path)
                └── no
                     ↓
                [subscription_mapper.exists_for_user(owner_id)?]
                     ├── yes ──→ webhook safety-net path:
                     │            regenerate now, write column, return blob
                     │            (log warning; webhook delayed/missed)
                     └── no  ──→ 404 (never Pro; feature not part of Free)
```

### Flow C — Stripe webhook: Pro → Free

```
[customer.subscription.deleted OR .updated to canceled]
        │
        ▼
[Existing: signature verify, idempotency on stripe_event_id, subscription mapper update]
        │
        ▼
[NEW: if old_tier=Paid and new_tier=Free:
   frozen_ics.regenerate_for_user(user_id)        # one blob per calendar
   invalidate user_settings cache]
        │
        ▼
[200 to Stripe]
```

`regenerate_for_user` is idempotent — Stripe retries are safe. Per-calendar AniList failure is logged but does not block other calendars; the safety-net path at next subscribe poll covers any holes.

### Flow D — Stripe webhook: Free → Pro

```
[customer.subscription.created OR status flips to active/trialing]
        │
        ▼
[Existing: subscription upsert]
        │
        ▼
[NEW: frozen_ics.clear_for_user(user_id)         # nulls all frozen blobs]
        │
        ▼
[200 to Stripe]
```

After this, all subscribe URLs owned by the user serve live data on the next poll. Tokens unchanged.

### Flow E — Item fetch via `CachedDataSource`

```
[caller: data_source.fetch_by_ids([1, 2, 3])]
        │
        ▼
[for each id:
   meta_hit  = cache.get("item:meta:{id}")    # 24h TTL
   airing_hit = cache.get("item:airing:{id}") # 15m TTL
   if both hit → assemble Item from cache; mark id satisfied
   else        → mark id missing]
        │
        ▼
[if any missing:
   inner.fetch_by_ids(missing_ids)            # one upstream call
   for each fetched item:
      cache.set("item:meta:{id}",   meta_part,   metadata_ttl)
      cache.set("item:airing:{id}", airing_part, airing_ttl)
   merge into result]
        │
        ▼
[Return Vec<Item> in caller's input order]
```

No cache stampede protection in v1. Two concurrent fetches for the same uncached ID will both hit upstream. Acceptable at our scale; revisit if AniList rate-limit pressure emerges.

### State machine — `calendars.frozen_subscribe_ics`

```
                  ┌──────────────────┐
                  │ NEVER POPULATED  │
                  │ (never been Pro) │
                  │ subscribe → 404  │
                  └──────────────────┘
                          │ user upgrades to Pro for the first time
                          ▼
   ┌──────────────┐                              ┌─────────────────┐
   │              │  user becomes Pro             │                 │
   │  NULL        │ ◄────────────────────────────│  POPULATED      │
   │  (live mode) │                               │  (frozen mode)  │
   │              │ ──────────────────────────►   │                 │
   └──────────────┘   user becomes Free           └─────────────────┘
                      → regenerate from current        │  ▲
                        calendar contents              │  │ user mutates
                                                       │  │ calendar while Free
                                                       │  │ → regenerate
                                                       └──┘
```

The `NEVER POPULATED` and `NULL (live mode)` states share the column value `NULL`; they're disambiguated by the `subscriptions.exists_for_user` history check.

## Error contracts

| Endpoint | Status | Body | When |
|---|---|---|---|
| `PUT /api/calendar` (new calendar, Free at calendar limit) | 402 | `{"error":"...","required_tier":"paid"}` | Calendar count ≥ `free_calendar_limit` |
| `PUT /api/calendar` (items-diff introduces new media_id, Free at show cap) | 402 | `{"error":"...","required_tier":"paid"}` | Distinct count ≥ `free_show_cap` and media_id not already tracked |
| `PUT /api/calendar` (items-diff includes already-tracked media_id) | 200 | normal payload | Idempotent — count unchanged |
| `PUT /api/calendar` (invalid `event_style`) | 400 | `{"error":"event_style_invalid"}` | Not in `{timed, all_day}` |
| `PUT /api/user/settings` (malformed reminders) | 400 | `{"error":"reminder_offsets_invalid"}` | length > 5 / value outside canonical set / non-integer |
| `PUT /api/user/settings` (Free, non-default reminders) | 200 | normal payload | Stored but ignored at .ics emission |
| `GET /api/calendars/subscribe/{token}` (token unknown OR Free + no subscription history) | 404 | empty | Anti-enumeration |
| `GET /api/calendars/subscribe/{token}` (Free + history + null + AniList down) | 5xx with stale fallback if available | — | Lazy regen failed |

All 402 responses use `Error::PaymentRequired { required_tier: "paid" }`, matching Track 4 Phase 5's accent enforcement (`feedback_tier_gated_apply_pattern`). Frontend routes 402 to `UpgradeInterruptModal` with reason codes `cap_calendars`, `cap_shows`, `pro_accent`.

## Concurrency

**Items cap race** — concurrent adds via multiple tabs/devices: per-user transactional lock around the count→insert sequence ensures exactly one writer wins when the cap is the boundary.

**Calendar count race** — same pattern, per-user lock around count→insert.

**Stripe webhook race** — Track 4's existing `stripe_events.stripe_event_id` idempotency key handles duplicates. The new `frozen_ics.regenerate*` operations are idempotent.

## Failure modes

| Failure | User-visible effect | Recovery |
|---|---|---|
| AniList briefly down | Calendar apps see slightly-stale data | Auto-recovers when AniList recovers |
| AniList down during downgrade | Some `frozen_subscribe_ics` columns null | Lazy regen on next poll, or reconcile loop |
| Redis down | Slower; AniList hit harder | Auto-recovers; rate-limit risk during outage |
| Postgres down | 503 across the app | Operator response |
| Stripe webhook missed | Tier drift | Track 4 reconcile loop catches up within an hour |
| Mid-tier-transition poll | May serve live for seconds after downgrade | Self-correcting |
| Concurrent multi-tab add at cap | One wins, one gets 402 | Frontend opens upgrade modal on the loser |

The cache layer is treated as **non-fatal** — `CachedDataSource` logs cache failures and proceeds to upstream. Calendar apps polling subscribe URLs are served stale data on AniList outages rather than 5xx storms.

## Testing strategy

Tests live in the same file as code under test (per CLAUDE.md). Frontend tests under `__tests__/` adjacent to components, with vitest + @vue/test-utils + jsdom.

### Backend

**Phase 0:**
- `cached_returns_from_cache_when_both_keys_present` (mock inner panics on call).
- `partial_miss_only_fetches_missing_ids`.
- `meta_ttl_separate_from_airing_ttl` (time-mocked).
- `redis_failure_falls_back_to_upstream`.
- `result_order_matches_caller_order`.

**Phase 1:**
- `assert_can_create_calendar_*` — Pro passes; Free under cap passes; Free at cap blocks.
- `assert_can_add_show_idempotent_on_already_tracked`.
- `assert_can_add_show_blocks_new_media_at_cap`.
- `count_distinct_excludes_duplicates`.
- `concurrent_add_at_cap_one_wins_one_402`.
- `subscribe_endpoint_pro_returns_live`.
- `subscribe_endpoint_free_with_blob_returns_blob`.
- `subscribe_endpoint_free_null_with_history_lazy_regen`.
- `subscribe_endpoint_free_no_history_returns_404`.
- `subscribe_token_unchanged_across_tier_transitions`.
- `regenerate_writes_current_state`, `regenerate_is_idempotent`.
- `webhook_downgrade_triggers_regen_for_all_user_calendars`.
- `webhook_upgrade_nulls_blob_for_all_user_calendars`.
- `webhook_anilist_failure_for_one_calendar_does_not_block_others`.

**Phase 2:**
- `pro_user_with_three_offsets_emits_three_valarms`.
- `free_user_always_emits_single_30min_valarm_regardless_of_stored_offsets` (critical anti-bypass test).
- `event_style_timed_with_air_time_emits_dtstart_with_time`.
- `event_style_timed_without_air_time_falls_back_to_value_date`.
- `event_style_all_day_always_emits_value_date`.
- `update_settings_rejects_six_reminders` / `rejects_uncanonical_offset` / `accepts_empty_array`.
- `update_settings_preserves_offsets_across_pro_to_free_transition`.

**Phase 3:**
- `locales_parity` (existing CI gate).
- `retired_locale_keys_absent`.
- `pricing_locale_keys_present`.

### Frontend

- `UpgradePage.spec`: renders `$2.99` / `$24.99`, six features, no AniList-sync / sharing copy.
- `MyCalendarsPage.spec`: shows counter for Free, hides for Pro, disables New button at cap.
- `EditorItemsPanel.spec` (+ mobile): show counter, 80% banner, hard-block.
- `PreferencesTab.spec`: chip toggling, 5-cap disabled-when-full, no-auto-deselect, Pro-locked overlay for Free.
- `CalendarSettingsForm.spec`: event-style toggle, tooltip mentions all-day fallback.
- `api.spec`: 402 with `required_tier=paid` opens `UpgradeInterruptModal` with correct reason code.

### Project-memory test gotchas to apply

- `feedback_uimodal_teleport_tests` — `attachTo: document.body` and query body, not wrapper.
- `feedback_test_history_pollution` — `createMemoryHistory` only.
- `feedback_account_tab_router_stub_coupling` — `PreferencesTab` changes may require updating `AccountPage.spec.ts` router stub.
- `feedback_smoke_test_timer_leak` — fake timers or trigger the error branch when `onMounted` schedules `router.push`.
- `feedback_jsdom_focusable_offsetparent` — don't filter focus-trap by `offsetParent !== null` in Teleport-using components.
- `feedback_axe_oklch_false_positives` — re-verify any axe-flagged contrast issues with the canvas/alpha-composite approach.

### Manual / E2E

- `docs/checklists/2026-05-track-4-release-readiness.md` — unchanged.
- `docs/checklists/2026-05-monetisation-stripe-price-update.md` — runs the price change rollout.
- A short additional manual list (cap reach, downgrade-and-poll, reminder customisation, event-style toggle) will be appended at plan-writing time, either as a third checklist or as a spec appendix.

### Coverage philosophy

- **Tier-gating: 100% branch coverage required.** Every (tier, count, action) tuple has a test.
- **`.ics` output: snapshot tests** for representative event shapes.
- **Cache layer: behaviour, not implementation tests.** Don't assert on cache key strings; assert observable behaviour.
- **Frontend: `data-testid` selectors only**, per CLAUDE.md.

## Open follow-ups (out of scope for this spec)

- Cache stampede protection (`tokio::sync::OnceCell` or single-flight) — add only if AniList rate-limit pressure becomes visible.
- Per-show reminder overrides — v2 if user feedback shows demand.
- Read-only public web view — v2.
- Subscription history caching (denormalised flag on user row) — only if `exists_for_user` query is profiled hot.
- Cache key versioned prefix — add post-launch (track via project memory).

## Cross-references

- Companion plan-writing target: `docs/superpowers/plans/2026-05-04-monetisation-refinement-plan.md`.
- Track 4 reference for entitlement model: `docs/superpowers/specs/2026-05-01-track-4-upgrade-flow-design.md`.
- Track 4 implementation patterns reused: tier enforcement (`feedback_tier_gated_apply_pattern`), reconcile loop (`feedback_reconcile_conditional_update`), Stripe webhook idempotency (`feedback_stripe_webhook_user_mapping`).
- AniList-data invariants: `feedback_anilist_item_no_status`, `feedback_derive_over_extend_cached_blobs`.
