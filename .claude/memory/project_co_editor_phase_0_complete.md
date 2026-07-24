---
name: Co-editor sharing — Phase 0 complete (2026-05-05 PM)
description: Spec 2 (co-editor sharing) implementation; Phase 0 fully closed — all 12 tasks shipped + verification gate green (281 server / 412 frontend tests, redocly lint, build). Phase 1 (invitation lifecycle) is next.
type: project
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
Spec 2 (co-editor sharing) implementation. **Phase 0 of 5 is fully complete (12/12 tasks).** Schema + entities + both mappers + authz spine + handler wiring + meta_version conditional bump + GET /calendars split shape (backend + frontend) + verification gate all shipped. Phase 1 (invitation lifecycle) is next.

**Spec:** `docs/superpowers/specs/2026-05-05-co-editor-sharing-design.md`
**Plan:** `docs/superpowers/plans/2026-05-05-co-editor-sharing.md`
**Branch:** `main` (per user direction; established workflow on this project).

## Done so far (commits in order)

| Task | Commit | Description |
|---|---|---|
| 0.1 | `bb2d684` | Migration: `calendars.meta_version INTEGER NOT NULL DEFAULT 1` |
| 0.2 | `eb1701d` | Migration: `calendar_editors` table (composite PK + 2 partial indexes) |
| 0.3 | `45f94d4` | Migration: `calendar_invitations` table (CITEXT email + 3 indexes + status check) |
| 0.4 | `6376dcc` | Entity: `CalendarEditor` struct |
| 0.5 | `3a24ab4` | Entity: `CalendarInvitation` + `InvitationStatus` enum |
| 0.5b | `462bab5` | Migration: fix FK widths BIGINT→INTEGER (calendar_editors + calendar_invitations) |
| 0.6 | `6216338` | Mapper: `CalendarEditorMapper` — 8 methods + 7 tests |
| 0.7 | `cd90e08` | Mapper: `CalendarInvitationMapper` — 11 methods + 10 tests |
| 0.8a | `bb9adab` | Test fix: `bump_expiry` flake from TIMESTAMP-vs-NaiveDateTime precision |
| 0.8 | `ac7f12c` | Service: `SharingAuthz::assert_can` + `assert_can_in_tx` (+ `Error::Forbidden`) — 9 tests |
| 0.9 | `3ce5973` | Wire `SharingAuthz` into PUT update branch + DELETE handlers; DI through main/server; test apps register the new web::Data |
| 0.10 | `bcdc5e4` | meta_version conditional bump on PUT (`update_calendar` + `_with`); +5 tests; entity field added |
| 0.11 (server) | `952c47e` | GET /calendars split shape: `{ owned: { data, pagination }, shared_with_me }`; mapper `list_shared_with_user_with`; PageCalendar gains `editor_count`; SharedPageCalendar + CalendarOwner DTOs; +6 server tests |
| 0.11 (frontend) | `974b12a` | MyCalendarsPage two-section layout; new SharedCalendarTile; CalendarTile gains editor_count chip; sharing.* locale namespace; +5 frontend tests |
| 0.12 | `a635ccb` | Verification gate green; plan status table updated to reflect Phase 0 done. |

281 server / 412 frontend tests pass. Pedantic+nursery clippy clean. Redocly lint validates the OpenAPI spec. Frontend lint + build green.

## Plan-vs-reality drift observed (bake into next briefs)

The plan was drafted before fully reading the codebase. Real conventions:

1. **ID type is `i32`, not `i64`.** Calendar.id, Calendar.user_id, all FKs use `i32`. CalendarInvitation `id` is `i64` (BIGSERIAL), but its calendar_id and inviter_id are `i32`.
2. **Calendar's owner FK is `user_id`, not `owner_id`.** Plan said owner_id everywhere — wrong.
3. **Module aggregator is `server/src/entity.rs` / `mappers.rs` / `services.rs` (flat files), NOT `mod.rs` files.**
4. **Mappers use `sqlx::query!` + manual struct construction, not `query_as!` + `FromRow`.** Custom enum columns need `as "col: Enum"` cast in SELECT and `as Enum` (Rust cast) in bind sites — different syntax, same query.
5. **Mappers wrap a `Database` struct** (custom `PgPool` wrapper). Constructor: `pub async fn new(config: DatabaseConfig) -> ServerResult<Self>`. Test ctor: `pub const fn from_pool(pool: PgPool) -> Self` under `#[cfg(test)]`.
6. **Naming for transaction-shared helpers is mixed:** `CalendarMapper` uses `_with(conn,...)`, `SubscriptionMapper`/new mappers use `_in_tx(conn,...)`. Use `_in_tx` for new mappers.
7. **No `seed_user`/`seed_calendar` test helpers.** Tests call existing mapper static helpers directly: `UserMapper::create_user_with(&mut *tx, ...)`, `CalendarMapper::insert_calendar_with(&mut tx, Calendar { id: 0, ..defaults })`. `test_pool()` and `test_tx()` exist in `server/src/test_helpers.rs`.
8. **No `SubscriptionMapper::is_paid`.** Use `EntitlementService::effective_tier(user_id) -> Tier` (or `effective_tier_in_tx`). `Tier::{Free, Paid}` is the existing enum at `server/src/services/entitlement.rs`.
9. **`Error::Forbidden` was added in commit `ac7f12c`** (HTTP 403). See `feedback_error_variant_for_authz_failure`.
10. **`Claims` has only `sub: String`.** Use `claims.user_id() -> Result<i32, Error>` to get the i32. Services take `i32` directly, not `&Claims`.
11. **Migration drift was already RESOLVED on 2026-04-30** — `sqlx migrate run` works cleanly for new migrations.
12. **Plan's Task 0.10 named columns `accent` + a method `update_meta`** — wrong. The actual mutating columns are `name`, `language`, `event_style`; the actual functions are `update_calendar` + `update_calendar_with`. (`accent` is a user-preference column on `users`, not on calendars.)

## Task 0.10 design choices baked in

- **Same-param-in-two-contexts is sqlx-poison.** The conditional bump query's CASE WHEN clause uses **distinct parameters** (`$6/$7/$8`) for the comparison, with `name`/`event_style` cloned on the Rust side. Anchoring `$1` in both `SET name = $1` and `name <> $1` is rejected by Postgres PREPARE with "inconsistent types deduced for parameter $N", and explicit casts / IS DISTINCT FROM / table aliases all fail to fix it. See `feedback_sqlx_param_assignment_vs_comparison`.
- **`meta_version` is server-managed, never deserialized.** Mirrors the existing `frozen_subscribe_ics` pattern: `#[serde(skip_deserializing, default = "default_meta_version")]`. Request bodies that build a `CalendarEntity` (e.g. PUT handler) can set any value (we used `0`); the UPDATE doesn't read it.
- **`updated_at` always changes; `meta_version` only on content change.** Preserves the existing audit-style "row touched" semantics while making meta_version purely content-driven for SSE consumers in Phase 3.
- **6 SELECT/RETURNING sites updated** (4 in `mappers/calendar.rs`, 1 in `services/ics_export.rs`, the GROUP BY in `get_calendars_by_user_paginated_with` was extended too). `controllers/calendar.rs` PUT handler stub gets `meta_version: 0` (discarded).

## Pre-flight items still pending

Not yet done (deferred until first phase that needs them):
- **`[sharing]` config block.** Plan said add in pre-flight; deferred until Phase 1 actually needs the keys.
- **Email service confirmation.** Existing `lettre`-based path is wired (used by password-reset / verify-email flows). Phase 1 Task 1.4 reuses it.

## Task 0.11 design choices baked in

- **Pagination retained on `owned`, flat `shared_with_me`.** Spec was silent on pagination; dropping it would have meant a Pro power user with 50 cals × 50 items hammering Anilist with 2,500 ids in a cold-cache load — a real rate-limit concern. Keeping the existing paginated shape on `owned` preserves the current Anilist batch ceiling. `shared_with_me` is naturally bounded by per-calendar editor cap × accepted invitations and stays flat. Response shape: `{ owned: { data, pagination }, shared_with_me: [...] }`.
- **`users` schema gap:** the spec said `display_name` + `avatar_url`; reality is the `users` table has only `username`. Mapped `username → owner.display`, hardcoded `owner.avatar = null`. The wire shape is forward-compatible: when a profile column lands, only the SELECT changes, no API/frontend churn. Pattern documented inline in `entity::calendar::CalendarOwnerInfo`.
- **Single dedup'd Anilist batch covers both lists.** A user who edits a friend's calendar with overlapping shows pays one fetch for those ids regardless of which list they appear in.
- **Cache key reuse with Phase 2 TODO.** Combined response cached under existing `{user_id}:calendars:page:N:size:M` key. Owner-side mutations already invalidate via `cache.invalidate_user_paged_calendars`. Editor-mutation endpoints (Phase 2) will need to additionally invalidate the affected user's key set when their `shared_with_me` view changes — inline TODO in the handler.
- **Frontend split components.** Owned uses existing `CalendarTile` (now with `editor_count` chip). Shared uses new `SharedCalendarTile` (no owner-actions menu, just open + owner badge with avatar/initial fallback). Cap counter reads only `usage.calendarCount` (owner-side) — shared count doesn't push toward the cap.
- **Empty state when both lists are empty.** Single empty CTA invites creating the first calendar. The "My calendars" heading only renders when both sections appear, otherwise the page `<h1>` is enough context.

## Pick-up plan for next session

**Task 0.12 — Phase 0 verification gate.**

- [ ] Run full backend + frontend gates again: `cargo test --workspace`, `npm run test:unit`, `npm run lint`, `npm run build`. Per `feedback_parallel_verification_flakiness`, run sequentially, not in parallel.
- [ ] Update implementation status table at top of `docs/superpowers/plans/2026-05-05-co-editor-sharing.md`. Mark Phase 0 ✅ Done with the latest commit hash (`974b12a`).
- [ ] Verify the OpenAPI snapshot in CI (redocly lint) still passes — should, since the new types are registered in `openapi.rs`. If CI is flaky here, run `cargo run --quiet --bin openapi-export > openapi.json` locally and inspect manually.
- [ ] Commit `docs(plan): mark phase 0 complete`.

**Then:** Phase 1 — Invitation lifecycle (token mint/verify, send/accept/decline endpoints, anti-enumeration, rate limit, email service). Plan starts at line 928.

## Conventions confirmed in 0.6/0.7/0.8/0.9/0.10 (apply going forward)

- **Tx helper naming:** `_in_tx` for new mappers (older calendar.rs uses `_with` — leave it).
- **Public API shape:** instance methods wrap `crate::metrics::db::timed("name", ...)` block that delegates to `Self::xxx_in_tx(&mut *self.db.pool.acquire().await?, ...)`. Tests only ever exercise `_in_tx` via `test_tx()` rollback.
- **Test seeding:** `UserMapper::create_user_with(&mut *tx, ...)` and `CalendarMapper::insert_calendar_with(&mut tx, ...)` directly. For paid-tier seeding: raw `INSERT INTO subscriptions` with `tier='paid', status='active'` (mirrors `entitlement.rs::tests::make_paid`).
- **Dead-code:** Phase 0 code without Phase 1+ callers gets `#[allow(dead_code)]` with `// First production caller lands in Phase X` comment. Drop the allow + update the comment when wiring lands (as 0.9 did for assert_can).
- **Pedantic clippy:** `len as i64` → `i64::try_from(len).unwrap()`. Test helpers taking `Result<>` by value → take `&Result<>`. Doc-markdown lint flags `language/event_style` without backticks. Free-function `fn default_x() -> T { lit }` should be `const fn` (nursery `missing_const_for_fn`).
- **Custom enum round-trip in `query!`:** `SELECT status as "status: InvitationStatus"` (cast hint, quoted) on the way out; `$2, status as InvitationStatus` (Rust cast, unquoted) on the way in to UPDATEs.
- **Same-param in SET + comparison is poison.** Use distinct params (`$6/$7/$8`) and clone `String` fields on the Rust side. See `feedback_sqlx_param_assignment_vs_comparison`.
- **Extractor ordering vs auth middleware:** `web::Data<T>` extractors fire BEFORE the `Claims` extractor's auth check. If a 401-token test doesn't register an app_data the handler now extracts, the request 500s instead of 401-ing. Update no-token tests too when adding new web::Data params.
- **DI through main → start():** new services get constructed in `main.rs` after their dependencies, then passed as a positional arg to `server::server::start(...)`. Inside `start`, register via `.app_data(web::Data::new(svc.clone()))`.

## What this enables

After Phase 0 completes, the foundation is in place: tables, mappers, authorization spine wired into the existing handlers, meta_version cursor, and (next) split GET shape. Zero user-visible behaviour change. Phase 1 (invitation lifecycle) builds on top.
