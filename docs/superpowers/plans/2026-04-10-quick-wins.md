# Quick Wins Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eight isolated, low-risk improvements from the deep-dive audit — dead code removal, centralized constants, input validation, a network timeout, DB indexes, and a frontend image hint.

**Architecture:** Backend-first (Phase 1): make all Rust changes, verify the build, then commit. Frontend (Phase 2): two targeted Vue changes, verify the build, then commit. No dependencies between tasks within each phase.

**Tech Stack:** Rust/Actix-Web, sqlx migrations, Vue 3 + TypeScript, Vite

---

## File Map

| File | Change |
|------|--------|
| `server/src/error.rs` | Remove `NotImplemented` variant |
| `server/src/cache.rs` | Add `CACHE_TTL_*` constants |
| `server/src/controllers/calendar.rs` | Use TTL constants |
| `server/src/controllers/item.rs` | Use TTL constants |
| `server/src/controllers/items.rs` | Use TTL constants |
| `server/src/controllers/user.rs` | Fix `unwrap()` → `let Ok(...) else` |
| `server/src/controllers/auth.rs` | Add `is_valid_email()` + use in register |
| `anilist/src/client.rs` | Add 10s request timeout |
| `migrations/20260410000000_add_indexes.sql` | **New** — 3 `CREATE INDEX` statements |
| `frontend/src/components/shared/MediaItemCard.vue` | Add `loading="lazy"` |

---

## Phase 1 — Backend

### Task 1: Remove `NotImplemented` error variant

**Files:**
- Modify: `server/src/error.rs`

- [ ] **Step 1: Remove the variant declaration and its match arm**

  In `server/src/error.rs`, apply both edits:

  Remove the variant (line 21–22):
  ```rust
  // DELETE these two lines:
  #[error("Not Implemented Yet")] // TODO remove once done
  NotImplemented,
  ```

  Remove from `status_code()` match arm (currently grouped with `Database`, `Config`, etc.):
  ```rust
  // BEFORE:
  Self::Database(_)
  | Self::Config(_)
  | Self::Server(_)
  | Self::NotImplemented
  | Self::GovernorConfig => StatusCode::INTERNAL_SERVER_ERROR,

  // AFTER:
  Self::Database(_)
  | Self::Config(_)
  | Self::Server(_)
  | Self::GovernorConfig => StatusCode::INTERNAL_SERVER_ERROR,
  ```

- [ ] **Step 2: Verify no usages remain**

  Run:
  ```bash
  grep -rn "NotImplemented" server/src/
  ```
  Expected: no output.

- [ ] **Step 3: Check it compiles**

  Run:
  ```bash
  cd server && cargo check 2>&1 | head -20
  ```
  Expected: no errors (warnings about unused imports are fine at this stage).

---

### Task 2: Centralize cache TTL constants

**Files:**
- Modify: `server/src/cache.rs` (add constants)
- Modify: `server/src/controllers/calendar.rs` (replace magic numbers)
- Modify: `server/src/controllers/item.rs` (replace magic numbers)
- Modify: `server/src/controllers/items.rs` (replace magic numbers)

- [ ] **Step 1: Add constants to `cache.rs`**

  After the `use` imports (after line 5, before the `#[derive(Clone)]` struct), add:
  ```rust
  /// Cache TTL for item lookups and single-resource responses (1 hour).
  pub const CACHE_TTL_ITEM: u64 = 3600;
  /// Cache TTL for search results (30 minutes).
  pub const CACHE_TTL_SEARCH: u64 = 1800;
  /// Cache TTL for calendar export responses (2 hours).
  pub const CACHE_TTL_CALENDAR: u64 = 7200;
  ```

  Also replace the inline magic number in `CacheConfig::default()` (currently `ttl_seconds: 3600`):
  ```rust
  // BEFORE:
  ttl_seconds: 3600, // 1 hour default

  // AFTER:
  ttl_seconds: CACHE_TTL_ITEM,
  ```

- [ ] **Step 2: Update `controllers/calendar.rs`**

  Add to the top-level `use` block:
  ```rust
  use crate::cache::{Cache, CACHE_TTL_CALENDAR, CACHE_TTL_ITEM, CACHE_TTL_SEARCH};
  ```
  (Replace whatever `use crate::cache` line already exists.)

  Then replace all four inline `let cache_ttl = ...` assignments:
  ```rust
  // line ~128 (calendar export, 7200):
  let cache_ttl = CACHE_TTL_CALENDAR;

  // line ~210 (subscription token endpoint, 3600):
  let cache_ttl = CACHE_TTL_ITEM;

  // line ~392 (search results, 1800):
  let cache_ttl = CACHE_TTL_SEARCH;

  // line ~463 (items fetch, 3600):
  let cache_ttl = CACHE_TTL_ITEM;
  ```
  Also remove the inline comments (`// 2 hours`, `// 30 minutes`, etc.) — the constant names are self-documenting.

- [ ] **Step 3: Update `controllers/item.rs`**

  Add to imports:
  ```rust
  use crate::cache::{Cache, CACHE_TTL_ITEM};
  ```

  Replace:
  ```rust
  // BEFORE:
  let cache_ttl = 3600; // 1 hour

  // AFTER:
  let cache_ttl = CACHE_TTL_ITEM;
  ```

- [ ] **Step 4: Update `controllers/items.rs`**

  Add to imports:
  ```rust
  use crate::cache::{Cache, CACHE_TTL_ITEM, CACHE_TTL_SEARCH};
  ```

  Replace two assignments:
  ```rust
  // First occurrence (~line 38, items by IDs, 3600):
  let cache_ttl = CACHE_TTL_ITEM;

  // Second occurrence (~line 88, search results, 1800):
  let cache_ttl = CACHE_TTL_SEARCH;
  ```

- [ ] **Step 5: Verify no magic TTL numbers remain**

  Run:
  ```bash
  grep -n "cache_ttl = 3600\|cache_ttl = 1800\|cache_ttl = 7200" server/src/controllers/
  ```
  Expected: no output.

- [ ] **Step 6: Check it compiles**

  Run:
  ```bash
  cd server && cargo check 2>&1 | head -20
  ```
  Expected: no errors.

---

### Task 3: Fix `unwrap()` in user settings handlers

**Files:**
- Modify: `server/src/controllers/user.rs`

- [ ] **Step 1: Fix `get_user_settings` handler (line ~208)**

  ```rust
  // BEFORE:
  let user_id = claims
      .sub
      .parse::<i32>()
      .map_err(|_| Error::Unauthorised)
      .unwrap();

  // AFTER:
  let Ok(user_id) = claims.sub.parse::<i32>() else {
      return Error::Unauthorised.error_response();
  };
  ```

- [ ] **Step 2: Fix `update_user_settings` handler (line ~227)**

  ```rust
  // BEFORE:
  let user_id = claims
      .sub
      .parse::<i32>()
      .map_err(|_| Error::Unauthorised)
      .unwrap();

  // AFTER:
  let Ok(user_id) = claims.sub.parse::<i32>() else {
      return Error::Unauthorised.error_response();
  };
  ```

- [ ] **Step 3: Verify no `unwrap()` remain in hot paths**

  Run:
  ```bash
  grep -n "unwrap()" server/src/controllers/user.rs
  ```
  Expected: no output.

- [ ] **Step 4: Check it compiles**

  Run:
  ```bash
  cd server && cargo check 2>&1 | head -20
  ```
  Expected: no errors.

---

### Task 4: Email format validation in register

**Files:**
- Modify: `server/src/controllers/auth.rs`

- [ ] **Step 1: Write the failing test first**

  At the bottom of `server/src/controllers/auth.rs`, add:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_is_valid_email() {
          assert!(is_valid_email("user@example.com"));
          assert!(is_valid_email("a@b.co"));
          assert!(is_valid_email("user.name+tag@sub.domain.org"));
          assert!(!is_valid_email("notanemail"));
          assert!(!is_valid_email("@domain.com"));
          assert!(!is_valid_email("user@"));
          assert!(!is_valid_email("user@nodot"));
          assert!(!is_valid_email("user@@domain.com"));
          assert!(!is_valid_email("user@.com"));
          assert!(!is_valid_email("user@domain."));
          assert!(!is_valid_email(""));
      }
  }
  ```

- [ ] **Step 2: Run the test to verify it fails (function not defined yet)**

  Run:
  ```bash
  cd server && cargo test test_is_valid_email 2>&1 | tail -10
  ```
  Expected: compile error — `is_valid_email` not found.

- [ ] **Step 3: Add the `is_valid_email` helper**

  Above the `LoginRequest` struct (before line 11), add:
  ```rust
  fn is_valid_email(email: &str) -> bool {
      let parts: Vec<&str> = email.split('@').collect();
      if parts.len() != 2 {
          return false;
      }
      let (local, domain) = (parts[0], parts[1]);
      if local.is_empty() || domain.is_empty() {
          return false;
      }
      let dot_pos = domain.rfind('.');
      match dot_pos {
          None => false,
          Some(pos) => pos > 0 && pos < domain.len() - 1,
      }
  }
  ```

- [ ] **Step 4: Run the test to verify it passes**

  Run:
  ```bash
  cd server && cargo test test_is_valid_email 2>&1 | tail -5
  ```
  Expected:
  ```
  test controllers::auth::tests::test_is_valid_email ... ok
  test result: ok. 1 passed; 0 failed
  ```

- [ ] **Step 5: Call the validator in `register()`**

  In the `register` handler, after the `username` length check (after line 75), add:
  ```rust
  // Validate email format
  if !is_valid_email(&user_data.email) {
      return Error::InvalidRequest.error_response();
  }
  ```

- [ ] **Step 6: Re-run all server tests**

  Run:
  ```bash
  cd server && cargo test 2>&1 | tail -10
  ```
  Expected: all tests pass.

---

### Task 5: Anilist reqwest timeout

**Files:**
- Modify: `anilist/src/client.rs`

- [ ] **Step 1: Add `Duration` import and update `Client::new()`**

  Add to the `use` block at the top:
  ```rust
  use std::time::Duration;
  ```

  Replace `Client::new()`:
  ```rust
  // BEFORE:
  impl Client {
      #[must_use]
      pub fn new() -> Self {
          Self {
              client: ReqwestClient::new(),
          }
      }

  // AFTER:
  impl Client {
      #[must_use]
      pub fn new() -> Self {
          Self {
              client: ReqwestClient::builder()
                  .timeout(Duration::from_secs(10))
                  .build()
                  .expect("Failed to build Anilist HTTP client"),
          }
      }
  ```

- [ ] **Step 2: Verify it compiles**

  Run:
  ```bash
  cargo check --package anilist 2>&1 | head -20
  ```
  Expected: no errors.

---

### Task 6: DB indexes migration

**Files:**
- Create: `migrations/20260410000000_add_indexes.sql`

- [ ] **Step 1: Create the migration file**

  Create `migrations/20260410000000_add_indexes.sql` with:
  ```sql
  -- Add indexes on hot query paths identified in the performance audit.
  -- All three columns appear in WHERE or JOIN clauses on every request.

  CREATE INDEX IF NOT EXISTS idx_calendars_user_id
      ON calendars (user_id);

  CREATE INDEX IF NOT EXISTS idx_calendars_subscription_token
      ON calendars (subscription_token);

  CREATE INDEX IF NOT EXISTS idx_calendar_items_calendar_id
      ON calendar_items (calendar_id);
  ```

- [ ] **Step 2: Verify the file exists**

  Run:
  ```bash
  ls migrations/
  ```
  Expected: both `20260408000000_add_subscription_token.sql` and `20260410000000_add_indexes.sql` are listed.

---

### Task 7: Backend build + commit

- [ ] **Step 1: Full backend build**

  Run (Windows — requires NASM env var for `aws-lc-sys`):
  ```bash
  AWS_LC_SYS_PREBUILT_NASM=1 cargo build 2>&1 | tail -5
  ```
  Expected:
  ```
  Compiling server v0.1.0 (...)
  Finished `dev` profile ...
  ```
  If there are errors, fix them before proceeding.

- [ ] **Step 2: Run all server tests**

  Run:
  ```bash
  cargo test 2>&1 | tail -10
  ```
  Expected: all tests pass.

- [ ] **Step 3: Commit**

  ```bash
  git add server/src/error.rs \
          server/src/cache.rs \
          server/src/controllers/calendar.rs \
          server/src/controllers/item.rs \
          server/src/controllers/items.rs \
          server/src/controllers/user.rs \
          server/src/controllers/auth.rs \
          anilist/src/client.rs \
          migrations/20260410000000_add_indexes.sql
  git commit -m "fix: dead code, TTL constants, unwrap, email validation, anilist timeout, DB indexes"
  ```

---

## Phase 2 — Frontend

### Task 8: Image lazy loading

**Files:**
- Modify: `frontend/src/components/shared/MediaItemCard.vue`

- [ ] **Step 1: Add `loading="lazy"` to the `<img>` tag**

  In `MediaItemCard.vue`, around line 41–46:
  ```html
  <!-- BEFORE: -->
  <img
    :src="item.cover_image.medium"
    :alt="item.title.romaji"
    class="w-full h-full object-cover"
    @error="onImageError"
  />

  <!-- AFTER: -->
  <img
    :src="item.cover_image.medium"
    :alt="item.title.romaji"
    class="w-full h-full object-cover"
    loading="lazy"
    @error="onImageError"
  />
  ```

- [ ] **Step 2: Verify lint passes**

  Run:
  ```bash
  cd frontend && npm run lint 2>&1 | tail -5
  ```
  Expected: no errors.

---

### Task 9: Audit catch blocks (verification only — no code changes expected)

**Files:** (read-only review)

- [ ] **Step 1: Verify all user-facing catch blocks already handle errors**

  Run:
  ```bash
  grep -n "} catch" frontend/src/components/*.vue frontend/src/stores/*.ts frontend/src/config/api.ts
  ```

  Confirm each one falls into one of these categories:
  - Sets `error.value = t('...')` — user sees the error via the component's error display
  - Calls `toastService.error(...)` — user sees a toast
  - Is intentionally silent (sessionStorage read/write, config file load fallback)

  If any catch block silently swallows an error that would leave the UI in a broken state (blank page, stale data with no indication), add `error.value = t('errors.generic')` to it.

  Based on the pre-implementation review, all catches are already handled. This step is a confirm-and-skip.

---

### Task 10: Frontend build + commit

- [ ] **Step 1: Full frontend build**

  Run:
  ```bash
  cd frontend && npm run build 2>&1 | tail -10
  ```
  Expected: no TypeScript errors, build succeeds.

- [ ] **Step 2: Run frontend tests**

  Run:
  ```bash
  cd frontend && npm run test:unit 2>&1 | tail -10
  ```
  Expected: all tests pass.

- [ ] **Step 3: Commit**

  ```bash
  git add frontend/src/components/shared/MediaItemCard.vue
  git commit -m "perf: add loading=lazy to MediaItemCard cover images"
  ```

---

## Verification Checklist

After both phases are committed:

- [ ] `cargo build` passes with no new warnings about unused variables
- [ ] `cargo test` — the new `test_is_valid_email` test passes
- [ ] `npm run build` exits 0
- [ ] `npm run test:unit` exits 0
- [ ] `grep -rn "NotImplemented" server/src/` → no output
- [ ] `grep -n "cache_ttl = [0-9]" server/src/controllers/` → no output
- [ ] `grep -n "unwrap()" server/src/controllers/user.rs` → no output
- [ ] `ls migrations/` → both migration files present
