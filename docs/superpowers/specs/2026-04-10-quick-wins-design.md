# Quick Wins — Audit Follow-Up
**Date**: 2026-04-10
**Source**: `~/.claude/plans/deep-wondering-liskov.md`

## Scope

Eight isolated, low-risk improvements identified in the deep-dive audit. These have no dependencies on each other and require no schema redesign or new architecture.

---

## Phase 1 — Backend

### 1. Remove `NotImplemented` error variant
**File**: `server/src/error.rs`
Remove the `NotImplemented` variant and its match arm in `status_code()`/`error_response()`. OAuth is complete; this placeholder is dead code that returns an opaque 500.

### 2. Centralize cache TTL constants
**File**: `server/src/cache.rs`
Add three `pub const` values at module level:
```rust
pub const CACHE_TTL_ITEM: u64 = 3600;    // 1 hour
pub const CACHE_TTL_SEARCH: u64 = 1800;  // 30 minutes
pub const CACHE_TTL_CALENDAR: u64 = 7200; // 2 hours
```
Replace all inline magic numbers across `controllers/calendar.rs`, `controllers/item.rs`, `controllers/items.rs`.

### 3. Fix `unwrap()` in user settings handlers
**File**: `server/src/controllers/user.rs` lines 212, 231
Both sites already call `.map_err(|_| Error::Unauthorised)` before `.unwrap()`. Replace `.unwrap()` with `?` — the error type is already correct.

### 4. Email format validation in register
**File**: `server/src/controllers/auth.rs`
Before the DB lookup in the register handler, validate the email string: must contain exactly one `@`, have at least one character before it, and the part after `@` must contain a `.` with at least one character on each side. Return `Error::InvalidRequest` (existing variant, maps to HTTP 400) on failure. No new crate required.

### 5. Anilist reqwest timeout
**File**: `anilist/src/client.rs`
Change `Client::new()` to use `ReqwestClient::builder().timeout(Duration::from_secs(10)).build()`. Use `.expect("Failed to build HTTP client")` — a builder failure is a startup bug, not a runtime condition.

### 6. DB indexes migration
**File**: `server/migrations/20260410000000_add_indexes.sql`
```sql
CREATE INDEX IF NOT EXISTS idx_calendars_user_id
    ON calendars (user_id);

CREATE INDEX IF NOT EXISTS idx_calendars_subscription_token
    ON calendars (subscription_token);

CREATE INDEX IF NOT EXISTS idx_calendar_items_calendar_id
    ON calendar_items (calendar_id);
```

**Verification**: `cargo build` must pass before moving to Phase 2.

---

## Phase 2 — Frontend

### 7. Image lazy loading
**File**: `frontend/src/components/shared/MediaItemCard.vue`
Add `loading="lazy"` attribute to the `<img>` tag. No logic change.

### 8. Empty catch blocks — user-facing errors
**Files**: `CalendarPage.vue`, `MyCalendarsPage.vue`, `UserDetailsPage.vue`, `UserSettingsPage.vue`

Add `toastService.error(t('errors.generic'))` (or the appropriate i18n key) in catch blocks for:
- Delete operations (calendars, items)
- Save/update operations (user details, settings)
- Load operations where the page would appear broken

Leave bare `catch {}` for background/auto-save operations where silent failure is acceptable (e.g., sessionStorage writes).

---

## Out of Scope (deferred)

- `window.handleGoogleLogin` global — requires GSI SDK refactor, not a quick win
- JWT → httpOnly cookies — Phase 2 of security hardening
- TypeScript `strict: true` — would require fixing many implicit `any` sites first
- Email verification flow — depends on SMTP infrastructure not yet in place

---

## Success Criteria

- `cargo build` passes after Phase 1
- `npm run build` passes after Phase 2
- No new `unwrap()`/`expect()` introduced in hot paths
- All 3 DB indexes exist in a committed migration file
