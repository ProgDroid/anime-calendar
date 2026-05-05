# Co-Editor Sharing implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Pro-gated co-editor sharing for calendars: invitation lifecycle, owner-only meta + editor-allowed item mutations, real-time fan-out via Server-Sent Events on top of Upstash Redis Pub/Sub, suspend-on-downgrade with unbounded restore, and a 5-editor cap per calendar.

**Architecture:** Five sequential phases on top of one pre-flight. Phase 0 lays the schema + authorization spine. Phase 1 ships the invitation lifecycle with full security hardening. Phase 2 layers editor mutations + Members-tab UI. Phase 3 adds live sync (SSE + Pub/Sub + presence). Phase 4 wires tier transitions (suspend / restore / 402 / UpgradePage). Phase 5 lands the public `/invite/:token` page, E2E specs, and operational notes. Each phase produces a working, committable state.

**Tech Stack:** Rust (Actix-Web 4 + sqlx 0.8 + redis 1.0 + actix-web-lab for SSE + argon2 for token hashing + lettre/SMTP via existing email service), Vue 3.5 + TypeScript, Tailwind v4, Upstash Redis Pub/Sub.

**Date filed:** 2026-05-05
**Spec:** `docs/superpowers/specs/2026-05-05-co-editor-sharing-design.md`
**Estimated total effort:** ~2.5 weeks of focused work, 5 phases each independently shippable.

---

## Implementation status

| Phase | Status |
|-------|--------|
| Pre-flight | ⬜ Pending |
| Phase 0 — Schema + authorization spine | ⬜ Pending |
| Phase 1 — Invitation lifecycle | ⬜ Pending |
| Phase 2 — Editor mutations + Members tab | ⬜ Pending |
| Phase 3 — Live sync (SSE + Pub/Sub) | ⬜ Pending |
| Phase 4 — Tier transitions | ⬜ Pending |
| Phase 5 — Public landing + E2E + ops | ⬜ Pending |

---

## File map

**Backend — new:**
- `server/migrations/<n>_add_calendars_meta_version.sql`
- `server/migrations/<n+1>_create_calendar_editors.sql`
- `server/migrations/<n+2>_create_calendar_invitations.sql`
- `server/src/entity/calendar_editor.rs` — `CalendarEditor` entity + status helpers
- `server/src/entity/calendar_invitation.rs` — `CalendarInvitation` entity + status enum
- `server/src/mappers/calendar_editor.rs` — repository for `calendar_editors`
- `server/src/mappers/calendar_invitation.rs` — repository for `calendar_invitations`
- `server/src/services/sharing_authz.rs` — `assert_can(actor, calendar, action)` helper
- `server/src/services/invitation_token.rs` — mint + hash + verify
- `server/src/services/invitation_service.rs` — orchestrates send/accept/decline/resend
- `server/src/services/calendar_events.rs` — SSE event types + publisher
- `server/src/services/presence.rs` — heartbeat/scan/dedupe
- `server/src/services/sharing_email.rs` — invitation + restore transactional email send
- `server/src/controllers/sharing.rs` — invitation + editor-management HTTP routes
- `server/src/controllers/sse.rs` — `GET /calendars/:id/events`
- `server/src/rate_limit/per_user.rs` — custom `KeyExtractor` for per-user rate limiting
- `server/src/redis_pubsub.rs` — Pub/Sub fan-out wrapper

**Backend — modified:**
- `server/src/controllers/calendar.rs` — split per-item endpoints; meta_version bump on `PUT`; `GET /calendars` shape change
- `server/src/controllers/stripe.rs` — suspend-on-downgrade + restore-on-upgrade transactions
- `server/src/services/entitlement.rs` — add `Action::Invite` Pro check
- `server/src/services/reconcile.rs` — same suspend/restore logic for the safety-net loop
- `server/src/main.rs` — DI wiring + new routes + Redis pubsub task
- `config.toml`, `config.toml.dist` — `[sharing]` block (cap, expiry days, rate limits)

**Frontend — new:**
- `frontend/src/components/calendar/MembersTab.vue` — editor list + pending invites + invite button
- `frontend/src/components/calendar/InviteEditorModal.vue` — email input modal
- `frontend/src/components/shared/PresenceChip.vue` — viewer count + popover
- `frontend/src/components/InviteLandingPage.vue` — public route handler
- `frontend/src/composables/usePresence.ts` — SSE + heartbeat + event dispatch
- `frontend/src/services/sharingService.ts` — HTTP client for invitations + editors
- `frontend/src/stores/sharingStore.ts` — pinia store: members, pending invites, presence

**Frontend — modified:**
- `frontend/src/components/MyCalendarsPage.vue` — split into `My calendars` + `Shared with you`
- `frontend/src/components/calendar/CalendarSettingsForm.vue` — add Members tab
- `frontend/src/components/calendar/CalendarEditorViewDesktop.vue` — mount PresenceChip + collision banner
- `frontend/src/components/calendar/CalendarEditorViewMobile.vue` — same
- `frontend/src/components/UpgradePage.vue` — swap "Early access" → "Share calendars"
- `frontend/src/router/index.ts` — add public `/invite/:token` route
- `frontend/src/services/calendars.ts` — split per-item endpoints; new `GET /calendars` shape
- `frontend/src/types/calendar.ts` — `editor_count`, `owner` fields
- `frontend/src/locales/en.json`, `frontend/src/locales/pt.json` — `sharing.*` namespace; retire `upgrade.proFeatures.earlyAccess`
- `vitest.setup.ts` — global `EventSource` mock

**Tests — new:**
- `e2e/sharing-happy-path.spec.ts`
- `e2e/sharing-downgrade-kick.spec.ts`
- `frontend/src/components/calendar/__tests__/MembersTab.spec.ts`
- `frontend/src/components/calendar/__tests__/InviteEditorModal.spec.ts`
- `frontend/src/components/__tests__/InviteLandingPage.spec.ts`
- `frontend/src/components/shared/__tests__/PresenceChip.spec.ts`
- `frontend/src/composables/__tests__/usePresence.spec.ts`

**Docs — new:**
- `docs/checklists/2026-05-co-editor-sharing-uat.md`
- Update `docs/superpowers/specs/2026-04-20-deployment-design.md` (or successor) with token-scrubbing addendum.

---

## Pre-flight

Tasks that must complete before Phase 0 starts.

- [ ] **Confirm migration numbering.**
  Run `ls server/migrations/ | sort | tail -5`. Pick the next three sequential numbers. Use them throughout the plan as `<N>`, `<N+1>`, `<N+2>`.

- [ ] **Confirm Upstash Redis is reachable from dev.**
  ```bash
  redis-cli -u "$REDIS_URL" PING
  ```
  Expected: `PONG`. If this fails, fix `config.toml`'s `[redis]` block before continuing — Phase 3 depends on it.

- [ ] **Confirm transactional email service is wired.**
  Find the existing email-send call site (Track 4 password-reset / verify-email used `lettre` via `server/src/services/email.rs` or similar). Search:
  ```
  grep -rn "send_email\|EmailService\|lettre" server/src/
  ```
  Note the entry point — Phase 1 reuses it for invitation emails. If no service exists, the implementer adds one in Phase 1 Task 1.4.

- [ ] **Confirm CITEXT extension available.**
  Run in psql against dev DB:
  ```sql
  SELECT * FROM pg_extension WHERE extname = 'citext';
  ```
  If empty: Phase 0 migration will `CREATE EXTENSION IF NOT EXISTS citext;` first.

- [ ] **Add `[sharing]` config block.**
  Append to `config.toml.dist`:
  ```toml
  [sharing]
  editor_cap                  = 5      # max active+pending editors per calendar
  invitation_expiry_days      = 7
  invite_rate_limit_per_hour  = 20
  presence_ttl_seconds        = 60
  presence_heartbeat_seconds  = 30
  sse_heartbeat_seconds       = 25
  ```
  Mirror in local `config.toml`. Add corresponding fields to `Config` struct (`server/src/config/server.rs`) with `#[serde(default = "...")]` defaults so existing configs don't break.

- [ ] **Pre-flight commit.**
  ```bash
  git add config.toml.dist config.toml server/src/config/server.rs
  git commit -m "chore(config): scaffold [sharing] block for spec 2"
  ```

---

## Phase 0 — Schema + authorization spine

**Goal:** Tables, mappers, the `assert_can` helper, and the new `GET /calendars` shape exist and are tested. No invitations yet, no UI changes — but every existing mutation now flows through `assert_can`.

**Independent ship state:** Server compiles, all existing tests pass, `cargo sqlx prepare` succeeds. No user-visible behaviour change.

### Task 0.1 — Migration: `calendars.meta_version`

**Files:**
- Create: `server/migrations/<N>_add_calendars_meta_version.sql`

- [ ] **Step 1: Write the migration**

`server/migrations/<N>_add_calendars_meta_version.sql`:
```sql
ALTER TABLE calendars
    ADD COLUMN meta_version INTEGER NOT NULL DEFAULT 1;
```

- [ ] **Step 2: Apply the migration**

```bash
cd server && DATABASE_URL=... sqlx migrate run
```
Expected: migration applied; no errors.

- [ ] **Step 3: Verify column**

```bash
psql $DATABASE_URL -c "\d calendars" | grep meta_version
```
Expected: `meta_version | integer | not null default 1`.

- [ ] **Step 4: Commit**

```bash
git add server/migrations/<N>_add_calendars_meta_version.sql
git commit -m "feat(db): add calendars.meta_version for SSE ordering"
```

### Task 0.2 — Migration: `calendar_editors`

**Files:**
- Create: `server/migrations/<N+1>_create_calendar_editors.sql`

- [ ] **Step 1: Write the migration**

`server/migrations/<N+1>_create_calendar_editors.sql`:
```sql
CREATE TABLE calendar_editors (
    calendar_id   BIGINT      NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    user_id       BIGINT      NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
    active        BOOLEAN     NOT NULL DEFAULT TRUE,
    suspended_at  TIMESTAMP   NULL,
    joined_at     TIMESTAMP   NOT NULL DEFAULT NOW(),
    PRIMARY KEY (calendar_id, user_id)
);

CREATE INDEX idx_calendar_editors_active
    ON calendar_editors (calendar_id)
    WHERE active = true;

CREATE INDEX idx_calendar_editors_user_active
    ON calendar_editors (user_id)
    WHERE active = true;
```

- [ ] **Step 2: Apply, verify, commit** (same shape as 0.1)

```bash
cd server && DATABASE_URL=... sqlx migrate run
psql $DATABASE_URL -c "\d calendar_editors"
git add server/migrations/<N+1>_create_calendar_editors.sql
git commit -m "feat(db): create calendar_editors table"
```

### Task 0.3 — Migration: `calendar_invitations`

**Files:**
- Create: `server/migrations/<N+2>_create_calendar_invitations.sql`

- [ ] **Step 1: Write the migration**

```sql
CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE calendar_invitations (
    id             BIGSERIAL PRIMARY KEY,
    calendar_id    BIGINT       NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    inviter_id     BIGINT       NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
    invitee_email  CITEXT       NOT NULL,
    token_hash     TEXT         NOT NULL,
    status         TEXT         NOT NULL DEFAULT 'pending',
    expires_at     TIMESTAMP    NOT NULL,
    sent_at        TIMESTAMP    NOT NULL DEFAULT NOW(),
    resolved_at    TIMESTAMP    NULL,
    CONSTRAINT calendar_invitations_status_chk
        CHECK (status IN ('pending','accepted','declined','revoked','expired','suspended'))
);

CREATE UNIQUE INDEX idx_calendar_invitations_pending_unique
    ON calendar_invitations (calendar_id, lower(invitee_email))
    WHERE status = 'pending';

CREATE INDEX idx_calendar_invitations_calendar_status
    ON calendar_invitations (calendar_id, status);

CREATE INDEX idx_calendar_invitations_token_hash
    ON calendar_invitations (token_hash)
    WHERE status = 'pending';
```

- [ ] **Step 2: Apply, verify, commit**

```bash
cd server && DATABASE_URL=... sqlx migrate run
psql $DATABASE_URL -c "\d calendar_invitations"
git add server/migrations/<N+2>_create_calendar_invitations.sql
git commit -m "feat(db): create calendar_invitations table"
```

### Task 0.4 — Entity: `CalendarEditor`

**Files:**
- Create: `server/src/entity/calendar_editor.rs`
- Modify: `server/src/entity/mod.rs` (add `pub mod calendar_editor;`)

- [ ] **Step 1: Write the entity**

`server/src/entity/calendar_editor.rs`:
```rust
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct CalendarEditor {
    pub calendar_id: i64,
    pub user_id: i64,
    pub active: bool,
    pub suspended_at: Option<NaiveDateTime>,
    pub joined_at: NaiveDateTime,
}
```

- [ ] **Step 2: Re-export from mod.rs**

Append to `server/src/entity/mod.rs`:
```rust
pub mod calendar_editor;
```

- [ ] **Step 3: Verify it compiles**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo build -p server
```

- [ ] **Step 4: Commit**

```bash
git add server/src/entity/calendar_editor.rs server/src/entity/mod.rs
git commit -m "feat(entity): add CalendarEditor"
```

### Task 0.5 — Entity: `CalendarInvitation`

**Files:**
- Create: `server/src/entity/calendar_invitation.rs`
- Modify: `server/src/entity/mod.rs`

- [ ] **Step 1: Write entity + status enum**

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Revoked,
    Expired,
    Suspended,
}

#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct CalendarInvitation {
    pub id: i64,
    pub calendar_id: i64,
    pub inviter_id: i64,
    pub invitee_email: String,
    #[serde(skip)]
    pub token_hash: String,
    pub status: InvitationStatus,
    pub expires_at: NaiveDateTime,
    pub sent_at: NaiveDateTime,
    pub resolved_at: Option<NaiveDateTime>,
}
```

- [ ] **Step 2: Re-export, build, commit**

```bash
# add `pub mod calendar_invitation;` to entity/mod.rs
AWS_LC_SYS_PREBUILT_NASM=1 cargo build -p server
git add server/src/entity/calendar_invitation.rs server/src/entity/mod.rs
git commit -m "feat(entity): add CalendarInvitation + InvitationStatus enum"
```

### Task 0.6 — Mapper: `CalendarEditorMapper`

**Files:**
- Create: `server/src/mappers/calendar_editor.rs`
- Modify: `server/src/mappers/mod.rs`

Use `test_pool()` for tests (per `feedback_no_sqlx_test_use_test_pool`); never `#[sqlx::test]`.

- [ ] **Step 1: Write the failing test**

At the bottom of `server/src/mappers/calendar_editor.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_pool, seed_user, seed_calendar};

    #[tokio::test]
    async fn upsert_active_idempotent() {
        let pool = test_pool().await;
        let owner = seed_user(&pool, "owner@test").await;
        let editor = seed_user(&pool, "editor@test").await;
        let cal = seed_calendar(&pool, owner.id).await;
        let mapper = CalendarEditorMapper::new(pool.clone());

        mapper.upsert_active(cal.id, editor.id).await.unwrap();
        mapper.upsert_active(cal.id, editor.id).await.unwrap(); // no-op

        let editors = mapper.list_active(cal.id).await.unwrap();
        assert_eq!(editors.len(), 1);
        assert_eq!(editors[0].user_id, editor.id);
    }
}
```

- [ ] **Step 2: Run, verify it fails (compile error: undefined struct)**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server calendar_editor::tests::upsert_active_idempotent
```
Expected: FAIL — `cannot find struct CalendarEditorMapper`.

- [ ] **Step 3: Implement minimal mapper**

```rust
use sqlx::PgPool;
use crate::entity::calendar_editor::CalendarEditor;
use crate::error::ServerResult;

#[derive(Clone)]
pub struct CalendarEditorMapper {
    pool: PgPool,
}

impl CalendarEditorMapper {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self { Self { pool } }

    /// Insert a row or set `active=true, suspended_at=NULL` if it already exists.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn upsert_active(&self, calendar_id: i64, user_id: i64) -> ServerResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO calendar_editors (calendar_id, user_id, active, suspended_at)
            VALUES ($1, $2, true, NULL)
            ON CONFLICT (calendar_id, user_id) DO UPDATE
                SET active = true, suspended_at = NULL
            "#,
            calendar_id, user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List active editors for a calendar.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn list_active(&self, calendar_id: i64) -> ServerResult<Vec<CalendarEditor>> {
        sqlx::query_as!(
            CalendarEditor,
            r#"SELECT calendar_id, user_id, active, suspended_at, joined_at
               FROM calendar_editors
               WHERE calendar_id = $1 AND active = true
               ORDER BY joined_at"#,
            calendar_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }
}
```

- [ ] **Step 4: sqlx prepare + run test**

```bash
DATABASE_URL=... cargo sqlx prepare --workspace -- --all-targets
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server calendar_editor
```
Expected: PASS.

- [ ] **Step 5: Add remaining methods (TDD per method)**

For each of the methods below, repeat the pattern: write failing test, implement, run, verify.

Required methods on `CalendarEditorMapper`:

```rust
/// Mark editor inactive (soft, preserves row for restore).
pub async fn suspend_for_calendar(&self, calendar_id: i64) -> ServerResult<u64> { /* UPDATE ... SET active=false, suspended_at=NOW() WHERE calendar_id=$1 AND active=true; returns rows_affected */ }

/// Same but inside an existing transaction (per feedback_audit_lock_in_tx_consistency).
pub async fn suspend_for_calendar_in_tx(tx: &mut sqlx::PgConnection, calendar_id: i64) -> ServerResult<u64> { /* ... */ }

/// Restore all suspended editors for a calendar.
pub async fn restore_for_calendar_in_tx(tx: &mut sqlx::PgConnection, calendar_id: i64) -> ServerResult<u64> { /* UPDATE ... SET active=true, suspended_at=NULL WHERE calendar_id=$1 AND suspended_at IS NOT NULL */ }

/// Delete one editor (hard).
pub async fn remove(&self, calendar_id: i64, user_id: i64) -> ServerResult<u64> { /* DELETE returning rows_affected */ }

/// Same in tx.
pub async fn remove_in_tx(tx: &mut sqlx::PgConnection, calendar_id: i64, user_id: i64) -> ServerResult<u64> { /* ... */ }

/// Is this user an active editor on this calendar?
pub async fn is_active_editor(&self, calendar_id: i64, user_id: i64) -> ServerResult<bool> { /* SELECT EXISTS */ }

/// All calendars where this user is an active editor (for GET /calendars shared_with_me).
pub async fn list_calendars_for_user(&self, user_id: i64) -> ServerResult<Vec<i64>> { /* SELECT calendar_id ... */ }

/// Count active editors for a calendar (cap-counting, in tx).
pub async fn count_active_in_tx(tx: &mut sqlx::PgConnection, calendar_id: i64) -> ServerResult<i64> { /* SELECT COUNT(*) ... */ }
```

Each method gets one test minimum. Tests cover: insert idempotency, suspend (rows_affected), restore (idempotent — no suspended rows = 0 affected), remove (returns 0 if missing), is_active_editor true/false, list_calendars_for_user includes only active, count_active_in_tx matches list_active.length.

- [ ] **Step 6: Re-export from mappers/mod.rs**

```rust
pub mod calendar_editor;
```

- [ ] **Step 7: Run full mapper test suite**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server mappers::calendar_editor
```
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add server/src/mappers/calendar_editor.rs server/src/mappers/mod.rs server/.sqlx/
git commit -m "feat(mapper): CalendarEditor CRUD + suspend/restore + count"
```

### Task 0.7 — Mapper: `CalendarInvitationMapper`

**Files:**
- Create: `server/src/mappers/calendar_invitation.rs`
- Modify: `server/src/mappers/mod.rs`

Same TDD pattern as 0.6. Required methods:

```rust
pub async fn create(&self, calendar_id: i64, inviter_id: i64, invitee_email: &str, token_hash: &str, expires_at: NaiveDateTime) -> ServerResult<CalendarInvitation> { /* INSERT RETURNING * */ }

pub async fn create_in_tx(tx: &mut sqlx::PgConnection, ...) -> ServerResult<CalendarInvitation> { /* same */ }

pub async fn find_pending_by_token_hash(&self, token_hash: &str) -> ServerResult<Option<CalendarInvitation>> { /* WHERE token_hash=$1 AND status='pending' AND expires_at > NOW() */ }

pub async fn find_by_id(&self, id: i64) -> ServerResult<Option<CalendarInvitation>> { /* ... */ }

pub async fn list_pending_for_calendar(&self, calendar_id: i64) -> ServerResult<Vec<CalendarInvitation>> { /* WHERE calendar_id=$1 AND status='pending' ORDER BY sent_at DESC */ }

pub async fn count_pending_in_tx(tx: &mut sqlx::PgConnection, calendar_id: i64) -> ServerResult<i64> { /* COUNT(*) ... pending */ }

pub async fn count_pending_for_inviter_since(&self, inviter_id: i64, since: NaiveDateTime) -> ServerResult<i64> { /* For rate limiting */ }

pub async fn mark_resolved(&self, id: i64, status: InvitationStatus) -> ServerResult<u64> { /* UPDATE SET status=$2, resolved_at=NOW() WHERE id=$1 AND status='pending' */ }

pub async fn mark_resolved_in_tx(tx: &mut sqlx::PgConnection, id: i64, status: InvitationStatus) -> ServerResult<u64> { /* same */ }

pub async fn suspend_pending_for_calendar_in_tx(tx: &mut sqlx::PgConnection, calendar_id: i64) -> ServerResult<u64> { /* UPDATE ... status='suspended' WHERE status='pending' */ }

pub async fn restore_suspended_for_calendar_in_tx(tx: &mut sqlx::PgConnection, calendar_id: i64) -> ServerResult<u64> { /* UPDATE ... status='pending' WHERE status='suspended' AND expires_at > NOW() */ }

pub async fn bump_expiry(&self, id: i64, new_expires_at: NaiveDateTime) -> ServerResult<u64> { /* UPDATE expires_at WHERE id=$1 AND status='pending' */ }
```

Critical tests:
- Partial unique index forbids two pending invites for same `(calendar, lower(email))`.
- After revoke (mark_resolved → 'revoked'), creating a new pending invite for same email succeeds.
- `find_pending_by_token_hash` returns `None` for expired (manually set `expires_at = NOW() - INTERVAL '1 day'`).
- `count_pending_in_tx` excludes accepted/declined/revoked rows.

- [ ] Each method: failing test → impl → pass → commit.
- [ ] Final commit:
  ```bash
  git add server/src/mappers/calendar_invitation.rs server/src/mappers/mod.rs server/.sqlx/
  git commit -m "feat(mapper): CalendarInvitation CRUD + lifecycle helpers"
  ```

### Task 0.8 — `assert_can` authorization helper

**Files:**
- Create: `server/src/services/sharing_authz.rs`
- Modify: `server/src/services/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_pool, seed_user, seed_calendar, seed_subscription};

    #[tokio::test]
    async fn owner_can_meta_mutate() {
        let pool = test_pool().await;
        let owner = seed_user(&pool, "owner@test").await;
        let cal = seed_calendar(&pool, owner.id).await;
        let editor_mapper = crate::mappers::calendar_editor::CalendarEditorMapper::new(pool.clone());
        let sub_mapper = ...;
        let authz = SharingAuthz::new(editor_mapper, sub_mapper);
        let claims = Claims::for_test(owner.id, "owner@test");

        authz.assert_can(&claims, &cal, Action::MetaMutate).await.unwrap();
    }

    #[tokio::test]
    async fn editor_cannot_meta_mutate() { /* expects Err(Forbidden) */ }

    #[tokio::test]
    async fn editor_can_item_mutate() { /* seed editor row, then assert_can ItemMutate ok */ }

    #[tokio::test]
    async fn non_member_cannot_item_mutate() { /* expects Forbidden */ }

    #[tokio::test]
    async fn free_owner_cannot_invite() {
        // expects Err(PaymentRequired { reason: "share_calendar" })
    }

    #[tokio::test]
    async fn pro_owner_can_invite() { /* ok */ }
}
```

- [ ] **Step 2: Implement**

```rust
use crate::entity::calendar::Calendar;
use crate::entity::user::Claims;
use crate::error::{Error, ServerResult};
use crate::mappers::calendar_editor::CalendarEditorMapper;
use crate::mappers::subscription::SubscriptionMapper;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ItemMutate,
    MetaMutate,
    ManageEditors,
    Invite,
}

#[derive(Clone)]
pub struct SharingAuthz {
    editors: CalendarEditorMapper,
    subs: SubscriptionMapper,
}

impl SharingAuthz {
    #[must_use]
    pub const fn new(editors: CalendarEditorMapper, subs: SubscriptionMapper) -> Self {
        Self { editors, subs }
    }

    /// Authorize `actor` to perform `action` on `cal`.
    ///
    /// # Errors
    /// - `Error::Forbidden` if the actor lacks the role required.
    /// - `Error::PaymentRequired { reason: "share_calendar" }` if action is Invite and owner is Free.
    pub async fn assert_can(&self, actor: &Claims, cal: &Calendar, action: Action) -> ServerResult<()> {
        let is_owner = actor.id == cal.owner_id;
        match action {
            Action::ItemMutate => {
                if is_owner { return Ok(()); }
                if self.editors.is_active_editor(cal.id, actor.id).await? { return Ok(()); }
                Err(Error::Forbidden)
            }
            Action::MetaMutate | Action::ManageEditors => {
                if is_owner { Ok(()) } else { Err(Error::Forbidden) }
            }
            Action::Invite => {
                if !is_owner { return Err(Error::Forbidden); }
                if self.subs.is_paid(actor.id).await? { Ok(()) }
                else { Err(Error::PaymentRequired { reason: "share_calendar".into() }) }
            }
        }
    }
}
```

- [ ] **Step 3: Run + commit**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server sharing_authz
git add server/src/services/sharing_authz.rs server/src/services/mod.rs
git commit -m "feat(authz): SharingAuthz::assert_can helper"
```

### Task 0.9 — Wire `assert_can` into existing calendar mutation handlers

**Files:**
- Modify: `server/src/controllers/calendar.rs`
- Modify: `server/src/main.rs` (DI for SharingAuthz)

- [ ] **Step 1: Inject `SharingAuthz` into App data**

In `main.rs` startup:
```rust
let sharing_authz = SharingAuthz::new(editor_mapper.clone(), subscription_mapper.clone());
// ... .app_data(web::Data::new(sharing_authz))
```

- [ ] **Step 2: Replace ad-hoc owner checks**

In each existing handler that mutates a calendar:
- `PUT /calendars/:id` → `authz.assert_can(&claims, &cal, Action::MetaMutate).await?`
- `DELETE /calendars/:id` → `authz.assert_can(&claims, &cal, Action::ManageEditors).await?`
- existing add/remove-show endpoints (will be split in Phase 2 but for now) → `authz.assert_can(&claims, &cal, Action::ItemMutate).await?`

Remove the inline `if cal.owner_id != claims.id { return Err(Forbidden) }` lines.

- [ ] **Step 3: Run full server test suite**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test --workspace
```
Expected: all existing tests still pass (assert_can returns Ok for the owner — semantics unchanged).

- [ ] **Step 4: Commit**

```bash
git add server/src/controllers/calendar.rs server/src/main.rs
git commit -m "refactor(controllers): route calendar mutations through SharingAuthz"
```

### Task 0.10 — `meta_version` bump on `PUT /calendars/:id`

**Files:**
- Modify: `server/src/mappers/calendar.rs` — add `update_meta_in_tx` that bumps `meta_version`
- Modify: `server/src/controllers/calendar.rs`
- Test: same file

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn put_calendar_bumps_meta_version() {
    let pool = test_pool().await;
    let owner = seed_user(&pool, "o@t").await;
    let cal = seed_calendar(&pool, owner.id).await;
    assert_eq!(cal.meta_version, 1);

    let mapper = CalendarMapper::new(pool.clone());
    mapper.update_meta(cal.id, "New Name", &cal).await.unwrap();

    let reloaded = mapper.find_by_id(cal.id).await.unwrap().unwrap();
    assert_eq!(reloaded.meta_version, 2);
    assert_eq!(reloaded.name, "New Name");
}

#[tokio::test]
async fn put_calendar_no_op_does_not_bump() {
    // Same name + accent → meta_version stays 1
}
```

- [ ] **Step 2: Implement**

In `update_meta`, the SQL becomes:
```sql
UPDATE calendars
SET name = $2, accent = $3, event_style = $4,
    meta_version = CASE
        WHEN name <> $2 OR accent <> $3 OR event_style <> $4
        THEN meta_version + 1
        ELSE meta_version
    END
WHERE id = $1
RETURNING *
```

The handler now also needs to expose `meta_version` in `Calendar` entity (already there via `SELECT *`).

- [ ] **Step 3: Add `meta_version` to `Calendar` struct in `entity/calendar.rs`**

```rust
pub meta_version: i32,
```

- [ ] **Step 4: cargo sqlx prepare + tests + commit**

```bash
DATABASE_URL=... cargo sqlx prepare --workspace -- --all-targets
AWS_LC_SYS_PREBUILT_NASM=1 cargo test --workspace
git add server/src/mappers/calendar.rs server/src/entity/calendar.rs server/.sqlx/
git commit -m "feat(calendar): bump meta_version on PUT only when fields change"
```

### Task 0.11 — `GET /calendars` split shape

**Files:**
- Modify: `server/src/controllers/calendar.rs`
- Modify: `server/src/mappers/calendar.rs` (add `list_shared_with_user`)
- Modify: `frontend/src/services/calendars.ts`
- Modify: `frontend/src/types/calendar.ts`
- Modify: `frontend/src/components/MyCalendarsPage.vue`
- Test: `server/src/controllers/calendar.rs` + `frontend/src/__tests__/MyCalendarsPage.spec.ts`

- [ ] **Step 1: Server failing test**

```rust
#[tokio::test]
async fn get_calendars_returns_owned_and_shared() {
    let pool = test_pool().await;
    let alice = seed_user(&pool, "alice@t").await;
    let bob   = seed_user(&pool, "bob@t").await;
    let alice_cal = seed_calendar(&pool, alice.id).await;
    let bob_cal   = seed_calendar(&pool, bob.id).await;
    let editor_mapper = CalendarEditorMapper::new(pool.clone());
    editor_mapper.upsert_active(bob_cal.id, alice.id).await.unwrap();

    let resp = http_get_calendars_as(&alice).await;
    assert_eq!(resp.owned.len(), 1);
    assert_eq!(resp.owned[0].id, alice_cal.id);
    assert_eq!(resp.shared_with_me.len(), 1);
    assert_eq!(resp.shared_with_me[0].id, bob_cal.id);
    assert_eq!(resp.shared_with_me[0].owner.id, bob.id);
}
```

- [ ] **Step 2: Implement server**

`mappers/calendar.rs::list_shared_with_user`:
```sql
SELECT c.*, u.id AS owner_id, u.display_name AS owner_display, u.avatar_url AS owner_avatar
FROM calendars c
JOIN calendar_editors ce ON ce.calendar_id = c.id AND ce.user_id = $1 AND ce.active = true
JOIN users u ON u.id = c.owner_id
ORDER BY c.created_at
```

Controller:
```rust
#[derive(Serialize, ToSchema)]
struct CalendarsResponse {
    owned: Vec<CalendarWithEditorCount>,
    shared_with_me: Vec<CalendarWithOwner>,
}

pub async fn list_calendars(...) -> ServerResult<HttpResponse> {
    let owned = calendar_mapper.list_for_owner_with_editor_count(claims.id).await?;
    let shared = calendar_mapper.list_shared_with_user(claims.id).await?;
    Ok(HttpResponse::Ok().json(CalendarsResponse { owned, shared_with_me: shared }))
}
```

`CalendarWithEditorCount` extends `Calendar` with `editor_count: i64` from a subquery on `calendar_editors`.

- [ ] **Step 3: Frontend types**

`frontend/src/types/calendar.ts`:
```ts
export interface CalendarOwner { id: number; display: string; avatar: string | null }
export interface OwnedCalendar extends Calendar { editor_count: number }
export interface SharedCalendar extends Calendar { owner: CalendarOwner }
export interface CalendarsResponse { owned: OwnedCalendar[]; shared_with_me: SharedCalendar[] }
```

- [ ] **Step 4: Frontend service**

`frontend/src/services/calendars.ts`:
```ts
export async function listCalendars(): Promise<CalendarsResponse> {
  const { data } = await axios.get<CalendarsResponse>('/calendars')
  return data
}
```

- [ ] **Step 5: MyCalendarsPage failing component test**

```ts
import { createMemoryHistory, createRouter } from 'vue-router'
// ... per feedback_vue_router_mock_leaks_across_workers — never vi.mock vue-router

it('renders Shared with you only when shared list is non-empty', async () => {
  // mount with empty shared_with_me; assert no [data-testid="shared-with-me-section"]
  // remount with one item; assert section appears with one card
})
```

- [ ] **Step 6: Update `MyCalendarsPage.vue`**

Two `<section>` blocks: one for `owned`, one (conditional `v-if="shared.length > 0"`) for `shared`. Owned cards keep current shape + `+N editors` chip when `editor_count > 0`. Shared cards use a new `<SharedCalendarCard>` component (inline as scoped slot or separate file — implementer's choice). Cap counter (`editorCount / 3`) only counts `owned.length`.

- [ ] **Step 7: Add locale keys**

```json
// en.json, sharing namespace seeded
{
  "sharing": {
    "myCalendars": "My calendars",
    "sharedWithYou": "Shared with you",
    "owner": "Owner: {name}",
    "editorChip": "{count} editor | {count} editors"
  }
}
```
Mirror in `pt.json`.

- [ ] **Step 8: Run all tests + commit**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test --workspace
cd frontend && npm run test:unit
git add -A
git commit -m "feat: GET /calendars returns owned + shared_with_me; MyCalendarsPage split"
```

### Task 0.12 — Phase 0 verification gate

- [ ] **Run the full test suite both sides:**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test --workspace
cd frontend && npm run test:unit && npm run lint && npm run build
```
Expected: all green.

- [ ] **Update implementation status table at top of this file.** Mark Phase 0 ✅ Done with the latest commit hash.

```bash
git add docs/superpowers/plans/2026-05-05-co-editor-sharing.md
git commit -m "docs(plan): mark phase 0 complete"
```

---

## Phase 1 — Invitation lifecycle

**Goal:** Owners can invite, revoke, resend; invitees can preview, accept, decline. Full security hardening (anti-enumeration, per-inviter rate limit, email-bound, self-invite reject) in place. No editor-mutation API yet — accept just creates the `calendar_editors` row.

**Independent ship state:** Pro owner can invite Free invitee end-to-end via curl + email link. Frontend Members tab can be wired in Phase 2.

### Task 1.1 — Invitation token mint + hash

**Files:**
- Create: `server/src/services/invitation_token.rs`
- Modify: `server/Cargo.toml` (`argon2 = "0.5"` already there from password hashing — verify; `rand = "0.8"` for token bytes)

- [ ] **Step 1: Failing test**

```rust
#[test]
fn token_roundtrip_verifies() {
    let svc = InvitationToken::new();
    let raw = svc.mint();
    assert_eq!(raw.len(), 64); // 32 bytes hex-encoded
    let hash = svc.hash(&raw).unwrap();
    assert!(svc.verify(&raw, &hash).unwrap());
    assert!(!svc.verify("tampered", &hash).unwrap());
}
```

- [ ] **Step 2: Implement**

```rust
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use rand::RngCore;

pub struct InvitationToken;

impl InvitationToken {
    #[must_use] pub const fn new() -> Self { Self }

    /// Mint a 256-bit random token, hex-encoded.
    #[must_use]
    pub fn mint(&self) -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    /// Argon2id hash of the raw token, suitable for storage.
    /// # Errors
    pub fn hash(&self, raw: &str) -> ServerResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(raw.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// # Errors
    pub fn verify(&self, raw: &str, hash: &str) -> ServerResult<bool> {
        let parsed = PasswordHash::new(hash).map_err(|e| Error::Internal(e.to_string()))?;
        Ok(Argon2::default().verify_password(raw.as_bytes(), &parsed).is_ok())
    }
}
```

- [ ] **Step 3: Run + commit**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server invitation_token
git add server/src/services/invitation_token.rs server/src/services/mod.rs server/Cargo.toml
git commit -m "feat(svc): InvitationToken mint+hash+verify"
```

### Task 1.2 — Per-user rate limiter (custom KeyExtractor)

**Files:**
- Create: `server/src/rate_limit/per_user.rs`
- Modify: `server/src/rate_limit/mod.rs`

Reference: `feedback_actix_governor_path_exempt` shows the pattern.

- [ ] **Step 1: Implement**

```rust
use actix_governor::{KeyExtractor, GovernorConfigBuilder};
use actix_governor::governor::clock::DefaultClock;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct PerUserKey;

impl KeyExtractor for PerUserKey {
    type Key = String;
    type KeyExtractionError = actix_governor::KeyExtractionError;

    fn extract(&self, req: &actix_web::dev::ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        let claims = req
            .extensions()
            .get::<crate::entity::user::Claims>()
            .cloned()
            .ok_or_else(|| Self::KeyExtractionError::new("not authenticated").set_status_code(actix_web::http::StatusCode::UNAUTHORIZED))?;
        Ok(format!("user:{}", claims.id))
    }
}

#[must_use]
pub fn invite_rate_limiter(per_hour: u32) -> actix_governor::GovernorConfig<PerUserKey, DefaultClock> {
    GovernorConfigBuilder::default()
        .key_extractor(PerUserKey)
        .seconds_per_request(3600 / per_hour as u64)
        .burst_size(per_hour)
        .finish()
        .expect("rate limiter config")
}
```

- [ ] **Step 2: Test**

Use existing rate-limit integration test pattern: hit endpoint 21 times in <1s, assert 21st is 429.

- [ ] **Step 3: Commit**

```bash
git add server/src/rate_limit/
git commit -m "feat(rate): PerUserKey extractor for per-user invite limit"
```

### Task 1.3 — Email validation helper

**Files:**
- Create: `server/src/services/email_validation.rs`
- Modify: `server/src/services/mod.rs`

- [ ] **Step 1: Failing tests**

```rust
#[test] fn rejects_empty() { assert!(validate_email("").is_err()); }
#[test] fn rejects_no_at() { assert!(validate_email("foo").is_err()); }
#[test] fn rejects_too_long() { assert!(validate_email(&"a".repeat(260)).is_err()); }
#[test] fn accepts_basic() { assert!(validate_email("a@b.co").is_ok()); }
#[test] fn rejects_self(){ assert!(check_not_self("foo@bar.com", "FOO@bar.com").is_err()); }
```

- [ ] **Step 2: Implement**

```rust
const MAX_EMAIL_LEN: usize = 254;

pub fn validate_email(email: &str) -> ServerResult<()> {
    if email.is_empty() || email.len() > MAX_EMAIL_LEN {
        return Err(Error::Validation("invalid_email".into()));
    }
    // Use existing email regex/parser already present elsewhere; if none, use `email_address` crate.
    if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(Error::Validation("invalid_email".into()));
    }
    Ok(())
}

pub fn check_not_self(invitee: &str, owner: &str) -> ServerResult<()> {
    if invitee.eq_ignore_ascii_case(owner) {
        Err(Error::Validation("self_invite".into()))
    } else { Ok(()) }
}
```

- [ ] **Step 3: Commit**

### Task 1.4 — Sharing email service

**Files:**
- Create: `server/src/services/sharing_email.rs`
- Modify: `server/src/services/mod.rs`

Reuse existing `EmailService` (the one that sends password-reset / verify-email). If no shared email module exists yet, this task is the place to introduce it — single `send(to, subject, html, text)` interface.

- [ ] **Step 1: Failing test (using a mock SMTP / capture)**

```rust
#[tokio::test]
async fn invitation_email_includes_link_and_html_escapes_calendar_name() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let svc = SharingEmail::new(MockEmail::new(captured.clone()), "https://app.test");
    svc.send_invitation(
        "invitee@t",
        "Owner Name",
        "<script>alert(1)</script>My Cal",
        "TOKEN_RAW",
    ).await.unwrap();
    let sent = captured.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].body_html.contains("https://app.test/invite/TOKEN_RAW"));
    // HTML escape:
    assert!(!sent[0].body_html.contains("<script>"));
    assert!(sent[0].body_html.contains("&lt;script&gt;"));
}
```

- [ ] **Step 2: Implement**

```rust
pub struct SharingEmail<E: EmailSender> {
    sender: E,
    base_url: String,
}

impl<E: EmailSender> SharingEmail<E> {
    pub async fn send_invitation(&self, to: &str, owner_display: &str, calendar_name: &str, raw_token: &str) -> ServerResult<()> {
        let owner_safe = html_escape::encode_safe(owner_display);
        let cal_safe = html_escape::encode_safe(calendar_name);
        let link = format!("{}/invite/{}", self.base_url, raw_token);
        let html = include_str!("../email_templates/invitation.html")
            .replace("{owner}", &owner_safe)
            .replace("{calendar}", &cal_safe)
            .replace("{link}", &link);
        let text = format!("{owner_display} invited you to edit \"{calendar_name}\" on Anime Calendar.\n\nAccept here: {link}\n\nLink expires in 7 days.");
        self.sender.send(to, &format!("Edit access invite: {calendar_name}"), &html, &text).await
    }

    pub async fn send_restored(&self, to: &str, calendar_name: &str) -> ServerResult<()> {
        // Similar pattern, "Your editor access has been restored"
    }
}
```

`server/src/email_templates/invitation.html` is a small static template — no engine, just `replace()` for the three placeholders.

- [ ] **Step 3: Commit**

### Task 1.5 — Invitation service

**Files:**
- Create: `server/src/services/invitation_service.rs`

This is the orchestration layer the controller calls.

- [ ] **Step 1: Define service shape**

```rust
pub struct InvitationService {
    invitations: CalendarInvitationMapper,
    editors: CalendarEditorMapper,
    calendars: CalendarMapper,
    users: UserMapper,
    token: InvitationToken,
    email: SharingEmail<...>,
    pubsub: RedisPubSub,
    cap: i64,
    expiry_days: i64,
    pool: PgPool,  // for advisory-locked transactions
}
```

- [ ] **Step 2: Implement `send` with TDD**

The full body. This is the hardest service in the plan; implement carefully.

```rust
pub async fn send(&self, owner: &Claims, calendar_id: i64, invitee_email: &str) -> ServerResult<CalendarInvitation> {
    // 1. Load calendar, assert owner matches.
    let cal = self.calendars.find_by_id(calendar_id).await?
        .ok_or(Error::NotFound)?;
    if cal.owner_id != owner.id { return Err(Error::Forbidden); }

    // 2. Validate email shape, self-invite, and Pro tier (assert_can Action::Invite).
    crate::services::email_validation::validate_email(invitee_email)?;
    let owner_user = self.users.find_by_id(owner.id).await?
        .ok_or(Error::Internal("owner missing".into()))?;
    crate::services::email_validation::check_not_self(invitee_email, &owner_user.email)?;
    // Pro check is in assert_can Action::Invite — controller calls it before us.

    // 3. Mint token + hash up-front.
    let raw = self.token.mint();
    let hash = self.token.hash(&raw)?;
    let expires = chrono::Utc::now().naive_utc() + chrono::Duration::days(self.expiry_days);

    // 4. Open advisory-locked transaction; check cap; enforce uniqueness; insert.
    let mut tx = self.pool.begin().await?;
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext($1))", format!("invite:{calendar_id}"))
        .execute(&mut *tx).await?;

    let active = CalendarEditorMapper::count_active_in_tx(&mut *tx, calendar_id).await?;
    let pending = CalendarInvitationMapper::count_pending_in_tx(&mut *tx, calendar_id).await?;
    if active + pending >= self.cap {
        return Err(Error::Conflict { reason: "editor_cap_reached".into() });
    }

    // Partial unique index will reject duplicate pending → translate to friendly error.
    let invitation = match CalendarInvitationMapper::create_in_tx(
        &mut *tx, calendar_id, owner.id, invitee_email, &hash, expires
    ).await {
        Ok(i) => i,
        Err(Error::DbConstraint(name)) if name.contains("pending_unique") => {
            return Err(Error::Conflict { reason: "invite_already_pending".into() });
        }
        Err(e) => return Err(e),
    };
    tx.commit().await?;

    // 5. Send email AFTER commit. Failure logged but does not roll back.
    if let Err(e) = self.email.send_invitation(invitee_email, &owner_user.display_name, &cal.name, &raw).await {
        log::error!("invitation email send failed: {e:?}");
    }

    // 6. (Phase 3) publish Pub/Sub event — stub for now: no publish.
    Ok(invitation)
}
```

- [ ] **Step 3: Implement `accept`**

```rust
pub async fn accept(&self, actor: &Claims, raw_token: &str) -> ServerResult<CalendarInvitation> {
    // 1. Find pending by token_hash. To verify argon2 hash we'd need the candidate;
    //    instead store an HMAC-SHA256 prefix as the index AND argon2 for full verify.
    //    For v1 simplicity: SELECT all pending where expires_at > NOW() and verify each? NO — too slow.
    //    Solution: hash with deterministic SHA-256 prefix for index lookup; argon2id for verification.

    // We'll add a `token_lookup` column (SHA-256(token), 32 bytes hex) for index lookup, alongside argon2 token_hash.
    // See Phase 0 migration addendum below.

    let lookup = sha256_hex(raw_token);
    let inv = self.invitations.find_pending_by_lookup(&lookup).await?
        .ok_or(Error::Validation("invalid_invite_token".into()))?;

    // Constant-time verify
    if !self.token.verify(raw_token, &inv.token_hash)? {
        return Err(Error::Validation("invalid_invite_token".into()));
    }

    // Email-bound check
    let user = self.users.find_by_id(actor.id).await?.ok_or(Error::Unauthorised)?;
    if !user.email.eq_ignore_ascii_case(&inv.invitee_email) {
        return Err(Error::Forbidden);
    }

    // Atomic: mark accepted + upsert editor + commit.
    let mut tx = self.pool.begin().await?;
    let resolved = CalendarInvitationMapper::mark_resolved_in_tx(&mut *tx, inv.id, InvitationStatus::Accepted).await?;
    if resolved == 0 { return Err(Error::Validation("invalid_invite_token".into())); } // race: already resolved

    sqlx::query!(
        r#"INSERT INTO calendar_editors (calendar_id, user_id, active, suspended_at)
           VALUES ($1, $2, true, NULL)
           ON CONFLICT (calendar_id, user_id) DO UPDATE SET active=true, suspended_at=NULL"#,
        inv.calendar_id, actor.id
    ).execute(&mut *tx).await?;
    tx.commit().await?;

    Ok(inv)
}
```

> **Plan note:** the `token_lookup` column is a Phase 0 schema addition we missed — add it now via a follow-up migration in Task 1.5.5 below.

- [ ] **Step 4: Implement `decline`, `revoke`, `resend`, `preview`**

```rust
pub async fn decline(&self, actor: &Claims, raw_token: &str) -> ServerResult<()> { /* same lookup + email-bound + mark_resolved Declined */ }

pub async fn revoke(&self, owner: &Claims, invitation_id: i64) -> ServerResult<()> { /* find_by_id, assert ownership of the calendar, mark_resolved Revoked */ }

pub async fn resend(&self, owner: &Claims, invitation_id: i64) -> ServerResult<()> {
    // bump expires_at = NOW + 7d, re-send email with the SAME raw token if we still have it...
    // BUT we don't — we only stored the hash. So resend MUST mint a new token.
    // → mark old one revoked, create a new one, send email. (still rate-limit-counted)
}

pub async fn preview(&self, raw_token: &str) -> ServerResult<InvitationPreview> {
    let lookup = sha256_hex(raw_token);
    let inv = self.invitations.find_pending_by_lookup(&lookup).await?
        .ok_or(Error::Validation("invalid_invite_token".into()))?;
    if !self.token.verify(raw_token, &inv.token_hash)? {
        return Err(Error::Validation("invalid_invite_token".into()));
    }
    let cal = self.calendars.find_by_id(inv.calendar_id).await?.ok_or(Error::Validation("invalid_invite_token".into()))?;
    let owner = self.users.find_by_id(cal.owner_id).await?.ok_or(Error::Validation("invalid_invite_token".into()))?;
    let item_count = self.calendars.count_items(cal.id).await?;
    Ok(InvitationPreview {
        calendar_name: cal.name,
        owner_display: owner.display_name,
        owner_avatar: owner.avatar_url,
        item_count,
    })
}
```

- [ ] **Step 5: Tests for each method.** Coverage:
  - send: cap enforcement, self-invite reject, Pro reject (already in authz), duplicate-pending reject.
  - accept: success, wrong-email 403, expired 400, already-accepted 400, tampered token 400.
  - decline: success + email-bound.
  - revoke: only owner, idempotent (revoking already-revoked = 400).
  - resend: bumps expiry on a fresh row, email re-sent.
  - preview: success returns calendar+owner+count; any failure returns identical 400 shape.

- [ ] **Step 6: Commit**

### Task 1.5.5 — Migration follow-up: `token_lookup` column

**Files:**
- Create: `server/migrations/<N+3>_add_calendar_invitations_token_lookup.sql`

- [ ] **Step 1: Write migration**

```sql
ALTER TABLE calendar_invitations
    ADD COLUMN token_lookup TEXT;

-- Backfill is impossible (we don't have raw tokens), but no rows exist in prod yet — pre-release.
-- Make NOT NULL after a short grace.

CREATE UNIQUE INDEX idx_calendar_invitations_token_lookup_pending
    ON calendar_invitations (token_lookup)
    WHERE status = 'pending';
```

After deploy, follow-up migration to set NOT NULL once existing rows are populated. For pre-release, just:

```sql
ALTER TABLE calendar_invitations ALTER COLUMN token_lookup SET NOT NULL;
```

- [ ] **Step 2: Update mapper to write `token_lookup` on `create`**

```rust
pub async fn create_in_tx(tx: &mut PgConnection, calendar_id: i64, inviter_id: i64, invitee_email: &str, token_hash: &str, token_lookup: &str, expires_at: NaiveDateTime) -> ServerResult<CalendarInvitation> { /* INSERT ... token_lookup ... */ }

pub async fn find_pending_by_lookup(&self, lookup: &str) -> ServerResult<Option<CalendarInvitation>> { /* WHERE token_lookup=$1 AND status='pending' AND expires_at > NOW() */ }
```

- [ ] **Step 3: cargo sqlx prepare + tests + commit**

### Task 1.6 — Sharing controller

**Files:**
- Create: `server/src/controllers/sharing.rs`
- Modify: `server/src/main.rs`

- [ ] **Step 1: Routes**

```rust
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/calendars/{id}")
            .route("/invitations",         web::post().to(create_invitation))
            .route("/invitations/{iid}",   web::delete().to(revoke_invitation))
            .route("/invitations/{iid}/resend", web::post().to(resend_invitation))
            .route("/editors",             web::get().to(list_members))
            .route("/editors/me",          web::delete().to(leave_calendar))
            .route("/editors/{uid}",       web::delete().to(remove_editor)),
    )
    .service(
        web::scope("/invitations/{token}")
            .route("",                     web::get().to(preview_invitation))
            .route("/accept",              web::post().to(accept_invitation))
            .route("/decline",             web::post().to(decline_invitation)),
    );
}
```

The `/calendars/{id}/invitations` path uses the per-user rate limiter middleware (20/hr from config).

- [ ] **Step 2: Each handler — failing test → impl → pass.**

For brevity, here's the `create_invitation` handler:

```rust
#[utoipa::path(
    post, path = "/calendars/{id}/invitations",
    operation_id = "create_invitation",
    request_body = CreateInvitationRequest,
    responses((status = 201, body = CalendarInvitation), (status = 402), (status = 409)),
)]
pub async fn create_invitation(
    path: web::Path<i64>,
    body: web::Json<CreateInvitationRequest>,
    claims: web::ReqData<Claims>,
    authz: web::Data<SharingAuthz>,
    svc: web::Data<InvitationService>,
    calendars: web::Data<CalendarMapper>,
) -> ServerResult<HttpResponse> {
    let calendar_id = path.into_inner();
    let cal = calendars.find_by_id(calendar_id).await?.ok_or(Error::NotFound)?;
    authz.assert_can(&claims, &cal, Action::Invite).await?;
    let invitation = svc.send(&claims, calendar_id, &body.email).await?;
    Ok(HttpResponse::Created().json(invitation))
}
```

Anti-enumeration on token endpoints is delivered by `Error::Validation("invalid_invite_token")` mapping to `400 {"error":"invalid_invite_token"}` in the existing error→HTTP layer.

- [ ] **Step 3: Wire routes in `main.rs` with rate limiter applied to invitation creation only.**

```rust
.service(
    web::scope("")
        .wrap(actix_governor::Governor::new(&invite_rate_limiter(config.sharing.invite_rate_limit_per_hour)))
        .route("/calendars/{id}/invitations", web::post().to(create_invitation))
)
.configure(controllers::sharing::routes_unlimited)  // everything else
```

- [ ] **Step 4: cargo sqlx prepare + tests + commit**

### Task 1.7 — Phase 1 integration smoke

- [ ] **Step 1: Manual smoke**

```bash
# Pro user owner@test, fresh user invitee@test
curl -X POST http://localhost:8080/api/calendars/1/invitations \
  -H "Cookie: token=..." -H "Content-Type: application/json" \
  -d '{"email":"invitee@test"}'
# → 201, mail server log shows email with /invite/<token> link
```

- [ ] **Step 2: Run all tests**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo test --workspace
```

- [ ] **Step 3: Update plan status, commit.**

---

## Phase 2 — Editor mutations + Members tab

**Goal:** Editors can add/remove items via split per-item endpoints. Owners can manage editors and pending invites in the Members tab. No live sync yet — UI updates require manual refresh.

**Independent ship state:** Pro owner invites Free user, Free user accepts via curl, Free user can call `POST /calendars/:id/items` and see effects on next page load.

### Task 2.1 — Split per-item endpoints (server)

**Files:**
- Modify: `server/src/controllers/calendar.rs`
- Modify: `server/src/mappers/calendar.rs` — `add_item_idempotent`, `remove_item_idempotent`

- [ ] **Step 1: Failing tests**

```rust
#[tokio::test]
async fn add_item_idempotent_second_call_returns_affected_false() { /* ... */ }

#[tokio::test]
async fn remove_item_idempotent_missing_returns_affected_false() { /* ... */ }

#[tokio::test]
async fn editor_can_add_item() { /* seed editor, hit POST /items, assert 200 */ }

#[tokio::test]
async fn non_member_cannot_add_item() { /* assert 403 */ }
```

- [ ] **Step 2: Implement mapper methods**

```rust
pub async fn add_item_idempotent(&self, calendar_id: i64, media_id: i64) -> ServerResult<bool> {
    let r = sqlx::query!(
        "INSERT INTO calendar_items (calendar_id, media_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        calendar_id, media_id
    ).execute(&self.pool).await?;
    Ok(r.rows_affected() > 0)
}

pub async fn remove_item_idempotent(&self, calendar_id: i64, media_id: i64) -> ServerResult<bool> {
    let r = sqlx::query!("DELETE FROM calendar_items WHERE calendar_id=$1 AND media_id=$2", calendar_id, media_id)
        .execute(&self.pool).await?;
    Ok(r.rows_affected() > 0)
}
```

- [ ] **Step 3: Implement controllers**

```rust
pub async fn add_item(path: web::Path<i64>, body: web::Json<AddItemRequest>, claims: web::ReqData<Claims>, authz: web::Data<SharingAuthz>, calendars: web::Data<CalendarMapper>) -> ServerResult<HttpResponse> {
    let calendar_id = path.into_inner();
    let cal = calendars.find_by_id(calendar_id).await?.ok_or(Error::NotFound)?;
    authz.assert_can(&claims, &cal, Action::ItemMutate).await?;
    // Free-tier show-cap (existing entitlement) only applies to OWNER, not editor — owner-funded model.
    if claims.id == cal.owner_id {
        entitlement.assert_can_add_show(claims.id, /* current count */).await?;
    }
    let affected = calendars.add_item_idempotent(calendar_id, body.media_id).await?;
    Ok(HttpResponse::Ok().json(json!({ "affected": affected })))
}

pub async fn remove_item(...) -> ServerResult<HttpResponse> { /* analogous */ }
```

- [ ] **Step 4: Routes + commit**

### Task 2.2 — Frontend: split per-item endpoints

**Files:**
- Modify: `frontend/src/services/calendars.ts`
- Modify: `frontend/src/components/calendar/CalendarEditorViewDesktop.vue`
- Modify: `frontend/src/components/calendar/CalendarEditorViewMobile.vue`

- [ ] **Step 1: Service**

```ts
export async function addItem(calendarId: number, mediaId: number): Promise<{ affected: boolean }> {
  const { data } = await axios.post(`/calendars/${calendarId}/items`, { media_id: mediaId })
  return data
}
export async function removeItem(calendarId: number, mediaId: number): Promise<{ affected: boolean }> {
  const { data } = await axios.delete(`/calendars/${calendarId}/items/${mediaId}`)
  return data
}
```

- [ ] **Step 2: Replace the existing batch save with per-item calls in editor save flow.**

The editor's "Save" button should now diff added/removed items locally and fire one POST/DELETE per change. Loop in parallel with `Promise.all`. Show error toast on any failure.

- [ ] **Step 3: Component tests + commit**

### Task 2.3 — Members tab UI

**Files:**
- Create: `frontend/src/components/calendar/MembersTab.vue`
- Create: `frontend/src/components/calendar/InviteEditorModal.vue`
- Create: `frontend/src/stores/sharingStore.ts`
- Create: `frontend/src/services/sharingService.ts`
- Modify: `frontend/src/components/calendar/CalendarSettingsForm.vue`
- Test: `frontend/src/components/calendar/__tests__/MembersTab.spec.ts`, `InviteEditorModal.spec.ts`

- [ ] **Step 1: sharingStore (pinia)**

```ts
export const useSharingStore = defineStore('sharing', () => {
  const editors = ref<Member[]>([])
  const pendingInvites = ref<Invitation[]>([])
  async function loadMembers(calendarId: number) {
    const { editors: e, pending } = await sharingService.listMembers(calendarId)
    editors.value = e; pendingInvites.value = pending
  }
  async function invite(calendarId: number, email: string) {
    const inv = await sharingService.invite(calendarId, email)
    pendingInvites.value.unshift(inv)
  }
  async function revoke(calendarId: number, invitationId: number) {
    await sharingService.revoke(calendarId, invitationId)
    pendingInvites.value = pendingInvites.value.filter(i => i.id !== invitationId)
  }
  async function removeEditor(calendarId: number, userId: number) {
    await sharingService.removeEditor(calendarId, userId)
    editors.value = editors.value.filter(e => e.user_id !== userId)
  }
  async function resend(calendarId: number, invitationId: number) {
    const fresh = await sharingService.resend(calendarId, invitationId)
    pendingInvites.value = pendingInvites.value.map(i => i.id === invitationId ? fresh : i)
  }
  return { editors, pendingInvites, loadMembers, invite, revoke, removeEditor, resend }
})
```

- [ ] **Step 2: MembersTab.vue**

Layout per spec §6.2: Active editors list, Pending invites list, Invite button. Tab is mounted only when `current_user.id === calendar.owner_id`. For Free owner, render the existing `<UpgradeLockOverlay>` with `share_calendar` reason instead.

```vue
<template>
  <section v-if="!isPaid" class="relative">
    <UpgradeLockOverlay reason="share_calendar" />
    <!-- ghost preview content -->
  </section>
  <section v-else>
    <!-- active editors list -->
    <!-- pending invites list -->
    <UiButton @click="showInviteModal = true">{{ t('sharing.inviteButton') }}</UiButton>
    <InviteEditorModal v-model:open="showInviteModal" :calendar-id="calendar.id" />
  </section>
</template>
```

Each editor row: `<UiAvatar /> <span>{{ display }}</span> <UiButton @click="remove(...)">{{ t('sharing.remove') }}</UiButton>`.

Each pending row: `<span>{{ email }}</span> <span>sent {{ relTime(sent_at) }}</span> <span>expires in {{ relTime(expires_at) }}</span> <UiButton @click="resend">…</UiButton> <UiButton @click="revoke">…</UiButton>`.

- [ ] **Step 3: InviteEditorModal.vue**

Single email input with client-side RFC 5321 surface validation, submit button calls `sharingStore.invite`. On 402 (`share_calendar`) the global interrupt modal handles it. On 409 (`editor_cap_reached` / `invite_already_pending`) show inline error in the modal.

- [ ] **Step 4: Mount in CalendarSettingsForm**

Add a new tab in the existing tab strip: `members` (visible only to owner, including ghost-mode for Free owner).

- [ ] **Step 5: Locale keys (en + pt)**

```json
"sharing": {
  "tabTitle": "Members",
  "activeEditors": "Active editors",
  "pendingInvites": "Pending invites",
  "inviteButton": "+ Invite editor",
  "inviteModal": {
    "title": "Invite editor",
    "emailLabel": "Email address",
    "submit": "Send invitation",
    "cancel": "Cancel"
  },
  "remove": "Remove",
  "resend": "Resend",
  "revoke": "Revoke",
  "errors": {
    "editor_cap_reached": "This calendar already has 5 editors (active or invited). Remove someone or wait for an invite to expire.",
    "invite_already_pending": "There's already a pending invitation for that email.",
    "self_invite": "You can't invite yourself.",
    "invalid_email": "Enter a valid email address."
  }
}
```

- [ ] **Step 6: Tests**

Per `feedback_vue_router_mock_leaks_across_workers` — `createMemoryHistory`, never `vi.mock('vue-router')`.
Per `feedback_uimodal_teleport_tests` — `attachTo: document.body` for InviteEditorModal tests.

Coverage: owner sees full UI, editor doesn't see tab (controlled by parent), Free owner sees overlay, invite submit calls store action, pending appears, revoke removes from list, error keys map to inline messages.

- [ ] **Step 7: Commit**

### Task 2.4 — Leave-calendar action

**Files:**
- Modify: `frontend/src/components/MyCalendarsPage.vue`

- [ ] **Step 1: Add card menu with Leave action on shared cards.**

Existing card menu component (or new `<SharedCalendarMenu />`) with one item: `Leave calendar`. Click → confirmation dialog → `DELETE /calendars/:id/editors/me` → optimistically remove from `shared_with_me` list.

- [ ] **Step 2: Server endpoint already in place from 1.6.** Test the wiring end-to-end.

- [ ] **Step 3: Commit**

### Task 2.5 — Phase 2 verification gate

- [ ] **Run all tests both sides.**
- [ ] **Update plan status table, commit.**

---

## Phase 3 — Live sync (SSE + Pub/Sub)

**Goal:** Mutations on shared calendars propagate in real time to all viewers. Presence chip shows who's currently in the editor. Owner kick / leave / downgrade tear down editor streams cleanly.

**Independent ship state:** Two browsers on the same calendar see each other's adds/removes within 500 ms. No tier-transition kick logic yet (Phase 4).

### Task 3.1 — Redis Pub/Sub wrapper

**Files:**
- Create: `server/src/redis_pubsub.rs`
- Modify: `server/src/main.rs`

- [ ] **Step 1: Implement publisher + subscriber**

```rust
use redis::aio::ConnectionManager;
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct RedisPubSub {
    publisher: ConnectionManager,
    // Per-channel local broadcast: when our subscriber receives a message from Redis,
    // it fan-outs to all in-process SSE handlers via this map.
    subscribers: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
    sub_conn_url: String,
}

impl RedisPubSub {
    pub async fn new(url: &str) -> ServerResult<Self> { /* ... */ }

    pub async fn publish<T: Serialize>(&self, channel: &str, payload: &T) -> ServerResult<()> {
        let json = serde_json::to_string(payload)?;
        let mut conn = self.publisher.clone();
        redis::cmd("PUBLISH").arg(channel).arg(json).query_async(&mut conn).await?;
        Ok(())
    }

    /// Subscribe to a channel; returns a `broadcast::Receiver` that yields each message.
    pub async fn subscribe(&self, channel: &str) -> ServerResult<broadcast::Receiver<String>> {
        let mut subs = self.subscribers.write().await;
        let tx = subs.entry(channel.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(64);
            // Spawn a task that subscribes via redis and pushes into this broadcast.
            spawn_redis_subscribe_task(self.sub_conn_url.clone(), channel.to_string(), tx.clone());
            tx
        });
        Ok(tx.subscribe())
    }
}
```

- [ ] **Step 2: Test against containerised Redis**

```rust
#[tokio::test]
async fn publish_subscribe_roundtrip() {
    let url = std::env::var("REDIS_URL").unwrap();
    let ps = RedisPubSub::new(&url).await.unwrap();
    let mut rx = ps.subscribe("test:chan").await.unwrap();
    ps.publish("test:chan", &json!({"hello":"world"})).await.unwrap();
    let msg = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    assert!(msg.contains("hello"));
}
```

- [ ] **Step 3: Commit**

### Task 3.2 — Calendar event types + publisher

**Files:**
- Create: `server/src/services/calendar_events.rs`

- [ ] **Step 1: Event enum**

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CalendarEvent {
    ItemAdded   { media_id: i64, actor: String, v: i32, at: NaiveDateTime },
    ItemRemoved { media_id: i64, actor: String, v: i32, at: NaiveDateTime },
    MetaUpdated { fields: Vec<String>, actor: String, v: i32, at: NaiveDateTime },
    MemberJoined{ user_id: String, display: String, actor: String },
    MemberLeft  { user_id: String, actor: String, reason: String },
    Presence    { viewers: Vec<Viewer> },
    Kick        { reason: String },
}

pub struct CalendarEventPublisher {
    pubsub: RedisPubSub,
}

impl CalendarEventPublisher {
    pub async fn publish_calendar(&self, calendar_id: i64, event: &CalendarEvent) -> ServerResult<()> {
        self.pubsub.publish(&format!("cal:{calendar_id}"), event).await
    }
    pub async fn publish_kick(&self, user_id: i64, reason: &str) -> ServerResult<()> {
        self.pubsub.publish(&format!("kick:{user_id}"), &CalendarEvent::Kick { reason: reason.into() }).await
    }
    pub async fn publish_presence(&self, calendar_id: i64, viewers: Vec<Viewer>) -> ServerResult<()> {
        self.pubsub.publish(&format!("presence:{calendar_id}"), &CalendarEvent::Presence { viewers }).await
    }
}
```

- [ ] **Step 2: Hook publisher into existing mutation handlers**

After each successful commit:
- `add_item` → `publish_calendar(cal_id, &ItemAdded{...})`
- `remove_item` → `ItemRemoved`
- `PUT /calendars/:id` (when meta_version actually bumped) → `MetaUpdated`
- `accept_invitation` → `MemberJoined`
- `remove_editor` → `MemberLeft (reason="removed")` + `publish_kick(uid, "role_revoked")`
- `leave_calendar` → `MemberLeft (reason="left")`

Failure to publish does NOT roll back; just `log::error!`.

- [ ] **Step 3: Commit**

### Task 3.3 — Presence service

**Files:**
- Create: `server/src/services/presence.rs`

- [ ] **Step 1: Implementation**

```rust
pub struct PresenceService {
    redis: ConnectionManager,
    ttl_seconds: u64,
}

impl PresenceService {
    /// Refresh a viewer's heartbeat. Returns the current full viewer list (after dedup).
    pub async fn heartbeat(&self, calendar_id: i64, user_id: i64, display: &str) -> ServerResult<Vec<Viewer>> {
        let key = format!("presence:cal:{calendar_id}:user:{user_id}");
        let val = serde_json::to_string(&json!({"display":display,"since": Utc::now()}))?;
        let mut conn = self.redis.clone();
        redis::cmd("SET").arg(&key).arg(&val).arg("EX").arg(self.ttl_seconds).query_async(&mut conn).await?;
        self.list(calendar_id).await
    }

    pub async fn list(&self, calendar_id: i64) -> ServerResult<Vec<Viewer>> {
        let mut conn = self.redis.clone();
        let pattern = format!("presence:cal:{calendar_id}:user:*");
        let mut cursor: u64 = 0;
        let mut viewers = Vec::new();
        loop {
            let (next, batch): (u64, Vec<String>) =
                redis::cmd("SCAN").arg(cursor).arg("MATCH").arg(&pattern).arg("COUNT").arg(50)
                    .query_async(&mut conn).await?;
            for key in batch {
                if let Ok(val) = redis::cmd("GET").arg(&key).query_async::<String>(&mut conn).await {
                    if let Ok(parsed) = serde_json::from_str::<ViewerStored>(&val) {
                        let user_id: i64 = key.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        viewers.push(Viewer { user_id, display: parsed.display });
                    }
                }
            }
            if next == 0 { break; }
            cursor = next;
        }
        Ok(viewers)
    }

    pub async fn drop(&self, calendar_id: i64, user_id: i64) -> ServerResult<()> {
        let mut conn = self.redis.clone();
        redis::cmd("DEL").arg(format!("presence:cal:{calendar_id}:user:{user_id}"))
            .query_async(&mut conn).await?;
        Ok(())
    }
}
```

- [ ] **Step 2: Heartbeat endpoint**

```rust
#[post("/calendars/{id}/presence/heartbeat")]
pub async fn heartbeat(path: web::Path<i64>, claims: web::ReqData<Claims>, authz: web::Data<SharingAuthz>, presence: web::Data<PresenceService>, publisher: web::Data<CalendarEventPublisher>, calendars: web::Data<CalendarMapper>, users: web::Data<UserMapper>) -> ServerResult<HttpResponse> {
    let cal_id = path.into_inner();
    let cal = calendars.find_by_id(cal_id).await?.ok_or(Error::NotFound)?;
    authz.assert_can(&claims, &cal, Action::ItemMutate).await?; // any member
    let user = users.find_by_id(claims.id).await?.ok_or(Error::Unauthorised)?;
    let viewers = presence.heartbeat(cal_id, claims.id, &user.display_name).await?;
    publisher.publish_presence(cal_id, viewers).await.ok();
    Ok(HttpResponse::NoContent().finish())
}
```

- [ ] **Step 3: Commit**

### Task 3.4 — SSE handler

**Files:**
- Create: `server/src/controllers/sse.rs`
- Modify: `server/Cargo.toml` (add `actix-web-lab = "0.20"`)

- [ ] **Step 1: Implement**

```rust
use actix_web_lab::sse::{self, Sse};

pub async fn calendar_events(
    path: web::Path<i64>,
    claims: web::ReqData<Claims>,
    authz: web::Data<SharingAuthz>,
    pubsub: web::Data<RedisPubSub>,
    presence: web::Data<PresenceService>,
    calendars: web::Data<CalendarMapper>,
    config: web::Data<Config>,
) -> ServerResult<Sse<impl futures::Stream<Item = Result<sse::Event, std::convert::Infallible>>>> {
    let calendar_id = path.into_inner();
    let cal = calendars.find_by_id(calendar_id).await?.ok_or(Error::NotFound)?;
    authz.assert_can(&claims, &cal, Action::ItemMutate).await?;

    let mut cal_rx = pubsub.subscribe(&format!("cal:{calendar_id}")).await?;
    let mut presence_rx = pubsub.subscribe(&format!("presence:{calendar_id}")).await?;
    let mut kick_rx = pubsub.subscribe(&format!("kick:{}", claims.id)).await?;

    let heartbeat_secs = config.sharing.sse_heartbeat_seconds;
    let viewers = presence.list(calendar_id).await.unwrap_or_default();
    let initial_meta = json!({ "type": "meta_snapshot", "v": cal.meta_version });
    let initial_presence = serde_json::to_string(&CalendarEvent::Presence { viewers })?;

    let stream = async_stream::stream! {
        // Initial reconcile frames.
        yield Ok(sse::Event::Data(sse::Data::new(initial_meta.to_string())));
        yield Ok(sse::Event::Data(sse::Data::new(initial_presence)));

        let mut interval = tokio::time::interval(Duration::from_secs(heartbeat_secs));
        loop {
            tokio::select! {
                msg = cal_rx.recv() => {
                    if let Ok(payload) = msg {
                        yield Ok(sse::Event::Data(sse::Data::new(payload)));
                    }
                }
                msg = presence_rx.recv() => {
                    if let Ok(payload) = msg { yield Ok(sse::Event::Data(sse::Data::new(payload))); }
                }
                msg = kick_rx.recv() => {
                    if let Ok(payload) = msg {
                        yield Ok(sse::Event::Data(sse::Data::new(payload)));
                        break; // close stream after kick
                    }
                }
                _ = interval.tick() => {
                    yield Ok(sse::Event::Comment("ping".into()));
                }
            }
        }
    };

    Ok(Sse::from_stream(stream))
}
```

- [ ] **Step 2: Cross-instance integration test**

```rust
// One Actix instance on port A subscribes; another on B publishes.
// Containerised Redis shared between them.
#[tokio::test]
async fn cross_instance_fan_out() {
    let redis = std::env::var("REDIS_URL").unwrap();
    let app_a = spawn_test_server(&redis, port_a).await;
    let app_b = spawn_test_server(&redis, port_b).await;

    let mut client = open_eventsource(&format!("http://localhost:{port_a}/api/calendars/{cal_id}/events")).await;

    http_post(&format!("http://localhost:{port_b}/api/calendars/{cal_id}/items"), AddItemRequest { media_id: 12345 }).await;

    let frame = tokio::time::timeout(Duration::from_secs(2), client.next_event()).await.unwrap();
    assert!(frame.contains("\"type\":\"item_added\""));
    assert!(frame.contains("12345"));
}
```

- [ ] **Step 3: Wire route + commit**

### Task 3.5 — Frontend `usePresence` composable

**Files:**
- Create: `frontend/src/composables/usePresence.ts`
- Test: `frontend/src/composables/__tests__/usePresence.spec.ts`

- [ ] **Step 1: Failing test**

```ts
import { mount } from '@vue/test-utils'

beforeEach(() => {
  global.EventSource = MockEventSource as any
})

it('opens EventSource on mount and closes on unmount', () => {
  const wrapper = mount({ setup() { return { p: usePresence(42) } }, template: '<div/>' })
  expect(MockEventSource.lastInstance.url).toBe('/api/calendars/42/events')
  wrapper.unmount()
  expect(MockEventSource.lastInstance.closed).toBe(true)
})

it('dispatches item.added to local handler', async () => {
  const wrapper = mount({ setup() { const p = usePresence(42); return { p } }, template: '<div/>' })
  MockEventSource.lastInstance.emit('message', JSON.stringify({ type: 'item.added', media_id: 99, actor: 'u_2', v: 5 }))
  await flushPromises()
  expect(wrapper.vm.p.lastEvent.value).toMatchObject({ type: 'item.added', media_id: 99 })
})
```

- [ ] **Step 2: Implement**

```ts
import { ref, onMounted, onBeforeUnmount } from 'vue'

export function usePresence(calendarId: number) {
  const viewers = ref<Viewer[]>([])
  const metaVersion = ref(0)
  const lastEvent = ref<CalendarEvent | null>(null)
  let es: EventSource | null = null
  let heartbeatTimer: number | null = null

  const onFrame = (data: string) => {
    let frame: any
    try { frame = JSON.parse(data) } catch { return }
    lastEvent.value = frame
    switch (frame.type) {
      case 'meta_snapshot':  metaVersion.value = frame.v; break
      case 'presence':       viewers.value = frame.viewers; break
      case 'item.added':
      case 'item.removed':
      case 'meta.updated':   metaVersion.value = frame.v ?? metaVersion.value; break
      case 'kick':           // emit toast + redirect handled by host component via watcher on lastEvent
                              break
    }
  }

  onMounted(() => {
    es = new EventSource(`/api/calendars/${calendarId}/events`, { withCredentials: true })
    es.onmessage = (e) => onFrame(e.data)
    heartbeatTimer = window.setInterval(() => {
      axios.post(`/calendars/${calendarId}/presence/heartbeat`).catch(() => {})
    }, 30_000)
  })

  onBeforeUnmount(() => {
    es?.close()
    if (heartbeatTimer != null) window.clearInterval(heartbeatTimer)
  })

  return { viewers, metaVersion, lastEvent }
}
```

- [ ] **Step 3: Add global EventSource mock to vitest.setup.ts**

```ts
class MockEventSource {
  static lastInstance: MockEventSource
  static instances: MockEventSource[] = []
  url: string
  closed = false
  listeners: Record<string, ((e: any) => void)[]> = {}
  onmessage: ((e: any) => void) | null = null
  constructor(url: string, _opts?: any) {
    this.url = url
    MockEventSource.lastInstance = this
    MockEventSource.instances.push(this)
  }
  close() { this.closed = true }
  emit(name: string, data: string) {
    const e = { data }
    if (name === 'message' && this.onmessage) this.onmessage(e)
    ;(this.listeners[name] ?? []).forEach(l => l(e))
  }
  addEventListener(name: string, l: (e: any) => void) {
    (this.listeners[name] ??= []).push(l)
  }
}
;(global as any).EventSource = MockEventSource
```

- [ ] **Step 4: Commit**

### Task 3.6 — PresenceChip component

**Files:**
- Create: `frontend/src/components/shared/PresenceChip.vue`
- Test: `frontend/src/components/shared/__tests__/PresenceChip.spec.ts`

- [ ] **Step 1: Implementation**

```vue
<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Viewer } from '@/types/sharing'
import { useAuthStore } from '@/stores/auth'

const props = defineProps<{ viewers: Viewer[] }>()
const auth = useAuthStore()
const open = ref(false)

const others = computed(() => props.viewers.filter(v => v.user_id !== auth.user?.id))
const includesSelf = computed(() => props.viewers.some(v => v.user_id === auth.user?.id))
const label = computed(() => {
  if (others.value.length === 0) return t('sharing.presence.justYou')
  return includesSelf.value
    ? t('sharing.presence.youPlusN', { n: others.value.length })
    : t('sharing.presence.nViewing', { n: others.value.length })
})
</script>

<template>
  <div class="relative">
    <button @click="open = !open" data-testid="presence-chip">{{ label }}</button>
    <div v-if="open" data-testid="presence-popover">
      <div v-for="v in viewers" :key="v.user_id">{{ v.display }}</div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Tests** — viewer count, popover open/close, "you" deduplication.

- [ ] **Step 3: Commit**

### Task 3.7 — Wire presence + live events into editor views

**Files:**
- Modify: `frontend/src/components/calendar/CalendarEditorViewDesktop.vue`
- Modify: `frontend/src/components/calendar/CalendarEditorViewMobile.vue`

- [ ] **Step 1: Mount**

```ts
const { viewers, metaVersion, lastEvent } = usePresence(calendarId)
```

- [ ] **Step 2: Render `<PresenceChip :viewers="viewers" />` in the header.**

- [ ] **Step 3: React to `lastEvent`**

```ts
watch(lastEvent, (frame) => {
  if (!frame) return
  switch (frame.type) {
    case 'item.added':
      if (!items.value.some(i => i.media_id === frame.media_id)) {
        // server-side echoes to all subscribers including actor — suppress duplicate via actor==='self'
        if (frame.actor !== authStore.user?.id?.toString())
          items.value = [...items.value, /* fetch or stub */]
      }
      break
    case 'item.removed':
      items.value = items.value.filter(i => i.media_id !== frame.media_id)
      break
    case 'meta.updated':
      if (form.dirty.value && frame.v > localBaselineMetaVersion.value) {
        showCollisionBanner.value = true
      } else {
        // refresh baseline silently
        reloadCalendar()
      }
      break
    case 'member.joined': toast(t('sharing.toasts.joined', { name: frame.display })); break
    case 'member.left':   toast(t(`sharing.toasts.left.${frame.reason}`, { name: /* lookup */ })); break
    case 'kick':
      toast(t(`sharing.toasts.kick.${frame.reason}`), { type: 'error' })
      router.push('/my-calendars')
      break
  }
})
```

- [ ] **Step 4: Locale keys for `sharing.toasts.*` and `sharing.presence.*`** (en + pt).

- [ ] **Step 5: Tests** — collision banner appears on dirty form + `meta.updated`; kick redirects.

- [ ] **Step 6: Commit**

### Task 3.8 — Phase 3 verification gate

- [ ] **Manual smoke** — two browsers, same calendar; add show in A, see it in B within 1 s.
- [ ] **Cross-instance test passes** in CI (Redis container).
- [ ] **Update plan status, commit.**

---

## Phase 4 — Tier transitions

**Goal:** Owner downgrade suspends editors and pending invites + kicks live streams; upgrade restores. Free owner cannot invite (402 `share_calendar`). UpgradePage shows the new feature.

**Independent ship state:** Full lifecycle Stripe-driven downgrade → kick → re-upgrade → restore → email passes the manual UAT.

### Task 4.1 — Suspend-on-downgrade transaction

**Files:**
- Modify: `server/src/controllers/stripe.rs`
- Modify: `server/src/services/reconcile.rs`

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn downgrade_suspends_editors_and_invites_and_publishes_kicks() {
    let pool = test_pool().await;
    let pubsub_capture = MockPubSub::new();
    /* seed Pro owner with 2 owned calendars, each with 2 active editors and 1 pending invite */
    /* simulate webhook downgrade */

    /* Assert:
       - all calendar_editors rows are active=false, suspended_at IS NOT NULL
       - all calendar_invitations are status='suspended'
       - pubsub_capture has 4 kick:* publishes (2 cals x 2 editors)
       - pubsub_capture has 4 member.left publishes on cal:* channels
    */
}
```

- [ ] **Step 2: Implement helper inside webhook handler**

```rust
async fn suspend_owner_sharing_in_tx(tx: &mut PgConnection, owner_id: i64) -> ServerResult<Vec<KickRecord>> {
    let owned = sqlx::query!("SELECT id FROM calendars WHERE owner_id = $1", owner_id)
        .fetch_all(&mut **tx).await?;
    let mut kicks = Vec::new();
    for row in &owned {
        let editors_before = sqlx::query!(
            "SELECT user_id FROM calendar_editors WHERE calendar_id = $1 AND active = true",
            row.id
        ).fetch_all(&mut **tx).await?;
        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut **tx, row.id).await?;
        CalendarInvitationMapper::suspend_pending_for_calendar_in_tx(&mut **tx, row.id).await?;
        for e in editors_before {
            kicks.push(KickRecord { calendar_id: row.id, user_id: e.user_id });
        }
    }
    Ok(kicks)
}

// Called inside the existing downgrade transaction.
// AFTER commit, iterate kicks and publish.
```

- [ ] **Step 3: After commit, publish events**

```rust
for kick in kicks {
    publisher.publish_calendar(kick.calendar_id, &CalendarEvent::MemberLeft {
        user_id: kick.user_id.to_string(),
        actor: "system".into(),
        reason: "downgrade".into(),
    }).await.ok();
    publisher.publish_kick(kick.user_id, "owner_downgrade").await.ok();
}
```

- [ ] **Step 4: Mirror in `services/reconcile.rs`** so the safety-net loop applies the same logic when it catches drift.

- [ ] **Step 5: Commit**

### Task 4.2 — Restore-on-upgrade transaction

**Files:**
- Modify: `server/src/controllers/stripe.rs`
- Modify: `server/src/services/reconcile.rs`

- [ ] **Step 1: Failing test** — symmetric to 4.1.

- [ ] **Step 2: Implement**

```rust
async fn restore_owner_sharing_in_tx(tx: &mut PgConnection, owner_id: i64) -> ServerResult<Vec<RestoreRecord>> {
    let owned = sqlx::query!("SELECT id, name FROM calendars WHERE owner_id=$1", owner_id)
        .fetch_all(&mut **tx).await?;
    let mut restored = Vec::new();
    for row in &owned {
        let restored_users = sqlx::query!(
            "SELECT user_id FROM calendar_editors WHERE calendar_id=$1 AND suspended_at IS NOT NULL",
            row.id
        ).fetch_all(&mut **tx).await?;
        CalendarEditorMapper::restore_for_calendar_in_tx(&mut **tx, row.id).await?;
        CalendarInvitationMapper::restore_suspended_for_calendar_in_tx(&mut **tx, row.id).await?;
        for r in restored_users {
            restored.push(RestoreRecord { calendar_id: row.id, calendar_name: row.name.clone(), user_id: r.user_id });
        }
    }
    Ok(restored)
}
```

- [ ] **Step 3: After commit, send restore emails**

```rust
for r in restored {
    let user = users.find_by_id(r.user_id).await?.unwrap();
    sharing_email.send_restored(&user.email, &r.calendar_name).await.ok();
}
```

- [ ] **Step 4: Commit**

### Task 4.3 — `share_calendar` 402 routing on frontend

**Files:**
- Modify: `frontend/src/components/UpgradeInterruptModal.vue` (or wherever the existing 402 reason map lives)
- Add locale keys.

- [ ] **Step 1: Add reason → copy mapping**

```ts
const REASON_COPY: Record<string, { heading: string; description: string }> = {
  // ... existing
  share_calendar: {
    heading: t('interrupt.heading.share_calendar'),
    description: t('interrupt.description.share_calendar'),
  },
}
```

Locale keys (en + pt) under `interrupt.{heading,description}.share_calendar`.

- [ ] **Step 2: Test** — server returns 402 with `share_calendar`; modal shows that copy.

- [ ] **Step 3: Commit**

### Task 4.4 — UpgradePage feature swap

**Files:**
- Modify: `frontend/src/components/UpgradePage.vue`
- Modify: `frontend/src/locales/en.json`, `pt.json`

- [ ] **Step 1: Replace `earlyAccess` key with `shareCalendars`**

```ts
const PRO_FEATURE_KEYS = [
  'unlimitedCalendars',
  'unlimitedShows',
  'liveSubscribeUrl',
  'customisableReminders',
  'allAccents',
  'shareCalendars',  // was 'earlyAccess'
] as const
```

```json
// en.json
"upgrade": {
  "proFeatures": {
    "shareCalendars": "Share calendars with co-editors (up to 5 per calendar)"
    // remove "earlyAccess"
  }
}
```

- [ ] **Step 2: Update existing UpgradePage snapshot test to expect 6 features still, with the new key replacing earlyAccess.**

- [ ] **Step 3: Retire `earlyAccess` from both locale files. Run `vue-i18n-auditor` if available.**

- [ ] **Step 4: Commit**

### Task 4.5 — Phase 4 verification gate

- [ ] **Manual UAT — full lifecycle:**
  1. Pro owner with 1 active editor.
  2. Editor opens calendar → SSE connected.
  3. Trigger downgrade via test webhook or `set_subscription` CLI.
  4. Editor's tab gets `kick` toast + redirect.
  5. `/my-calendars` for editor: calendar gone from `Shared with you`.
  6. Re-upgrade owner.
  7. Editor receives email; `/my-calendars` shows calendar back in `Shared with you`.

- [ ] **Update plan status, commit.**

---

## Phase 5 — Public landing + E2E + ops

**Goal:** `/invite/:token` page handles all four states (unauth / mismatch / accept-ready / invalid). Two E2E specs (happy path + downgrade kick) pass in CI. UAT checklist + token-scrubbing addendum committed.

**Independent ship state:** Feature is fully launchable.

### Task 5.1 — `/invite/:token` route + landing component

**Files:**
- Create: `frontend/src/components/InviteLandingPage.vue`
- Modify: `frontend/src/router/index.ts`
- Test: `frontend/src/components/__tests__/InviteLandingPage.spec.ts`

- [ ] **Step 1: Router**

```ts
{
  path: '/invite/:token',
  name: 'invite-landing',
  component: () => import('@/components/InviteLandingPage.vue'),
  meta: { public: true },
}
```

- [ ] **Step 2: Component**

```vue
<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { sharingService } from '@/services/sharingService'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const token = route.params.token as string

const state = ref<'loading' | 'invalid' | 'unauth' | 'mismatch' | 'ready' | 'accepted'>('loading')
const preview = ref<InvitationPreview | null>(null)
const errorReason = ref('')

onMounted(async () => {
  try {
    preview.value = await sharingService.preview(token)
    if (!auth.isAuthenticated) { state.value = 'unauth'; return }
    const me = await auth.fetchUser()
    if (me.email.toLowerCase() !== preview.value!.invitee_email.toLowerCase()) state.value = 'mismatch'
    else state.value = 'ready'
  } catch {
    state.value = 'invalid'
  }
})

async function accept() {
  await sharingService.accept(token)
  state.value = 'accepted'
  router.push(`/calendars/${preview.value!.calendar_id}`)
}
async function decline() {
  await sharingService.decline(token)
  router.push('/my-calendars')
}
</script>

<template>
  <UiCard>
    <div v-if="state === 'loading'">…</div>
    <div v-else-if="state === 'invalid'" data-testid="invite-invalid">
      {{ t('sharing.landing.invalid') }}
    </div>
    <div v-else-if="state === 'unauth'" data-testid="invite-unauth">
      <!-- preview card + Sign in / Sign up CTAs with ?redirect= -->
    </div>
    <div v-else-if="state === 'mismatch'" data-testid="invite-mismatch">
      {{ t('sharing.landing.mismatch', { email: maskedEmail }) }}
    </div>
    <div v-else-if="state === 'ready'" data-testid="invite-ready">
      <!-- preview + Accept / Decline buttons -->
    </div>
  </UiCard>
</template>
```

> Note: `preview` does NOT include `invitee_email`. Add a server-side `masked_email` field (e.g. `f***@h***.com`) so the mismatch state can show *something* without exposing the full address. **Update `InvitationPreview` server struct accordingly** before this task is "done".

- [ ] **Step 3: Tests** — render each state with stubbed sharingService; assert distinct content.

- [ ] **Step 4: Commit**

### Task 5.2 — E2E happy path

**Files:**
- Create: `e2e/sharing-happy-path.spec.ts`

- [ ] **Step 1: Two-context Playwright spec**

```ts
import { test, expect } from './fixtures'

test('invite + accept + live add', async ({ ownerContext, editorContext, ownerPage, editorPage, db }) => {
  await ownerPage.goto('/calendars/1/settings')
  await ownerPage.click('[data-testid="tab-members"]')
  await ownerPage.click('[data-testid="invite-button"]')
  await ownerPage.fill('[data-testid="invite-email-input"]', 'editor@test')
  await ownerPage.click('[data-testid="invite-submit"]')
  await expect(ownerPage.locator('[data-testid="pending-invite-row"]')).toContainText('editor@test')

  // Pull token from DB (test-only helper).
  const rawToken = await db.getLatestRawToken('editor@test') // dev fixture: store raw alongside hash in test mode
  await editorPage.goto(`/invite/${rawToken}`)
  await expect(editorPage.locator('[data-testid="invite-ready"]')).toBeVisible()
  await editorPage.click('[data-testid="invite-accept"]')
  await editorPage.waitForURL(/\/calendars\/1$/)

  // Owner's tab sees member.joined toast + presence chip update.
  await expect(ownerPage.locator('[data-testid="presence-chip"]')).toContainText('You + 1')

  // Editor adds an item; owner sees it appear without refresh.
  await editorPage.fill('[data-testid="search-input"]', 'naruto')
  await editorPage.click('[data-testid="add-show-naruto"]')
  await expect(ownerPage.locator('[data-testid="calendar-item-naruto"]')).toBeVisible({ timeout: 5_000 })
})
```

> **Test-mode raw-token fixture:** in `cfg(test)` builds, store the raw token alongside `token_hash` in a memo column or in-memory map keyed by invitation ID, exposed via a test-only endpoint `GET /test/invitations/latest-by-email/:email`. Production builds never expose this.

- [ ] **Step 2: Commit**

### Task 5.3 — E2E downgrade kick

**Files:**
- Create: `e2e/sharing-downgrade-kick.spec.ts`

- [ ] **Step 1: Spec**

```ts
test('owner downgrade kicks editor', async ({ ownerPage, editorPage, db, testHooks }) => {
  // pre-seed: pro owner + active editor + both viewing the calendar
  await ownerPage.goto('/calendars/1')
  await editorPage.goto('/calendars/1')
  await expect(editorPage.locator('[data-testid="presence-chip"]')).toContainText('You + 1')

  await testHooks.downgradeUser(ownerId)

  await expect(editorPage.locator('[data-testid="toast"]')).toContainText('access has been removed')
  await editorPage.waitForURL(/\/my-calendars/)
  await expect(editorPage.locator('[data-testid="shared-with-me-section"]')).toHaveCount(0)
})
```

- [ ] **Step 2: Commit**

### Task 5.4 — Manual UAT checklist

**Files:**
- Create: `docs/checklists/2026-05-co-editor-sharing-uat.md`

- [ ] **Step 1: Write checklist** matching spec §7.5 sections:

```markdown
# Co-Editor Sharing — Manual UAT

**Date created:** 2026-05-XX
**Owner:** _assign on first run_
**Source spec:** ../superpowers/specs/2026-05-05-co-editor-sharing-design.md

## 0. Prerequisites
- [ ] All five phases merged + deployed to staging.
- [ ] Two test accounts: `owner@test` (Pro) and `invitee@test` (Free).
- [ ] Real email inboxes for both (or Mailhog).

## 1. Invite happy path
- [ ] Pro owner adds Members tab → invites invitee@test → email arrives.
- [ ] Invitee opens link → preview page → accepts → lands on calendar editor.
- [ ] Owner's tab sees member.joined toast + presence chip "You + 1".

## 2. Invite expiry
- [ ] Set `expires_at = NOW() - INTERVAL '1 day'` on a pending row in psql.
- [ ] Hit GET /invitations/:token → `400 invalid_invite_token`.
- [ ] Frontend shows the generic invalid page.

## 3. Cap enforcement
- [ ] With 5 active+pending editors, attempt 6th invite → server 409 + frontend inline error.

## 4. Anti-enumeration
```bash
for status in not_found expired accepted revoked; do
  curl -s "http://staging/api/invitations/SCENARIO_$status" -o /tmp/$status.json
done
# All four bodies must be byte-identical.
diff /tmp/{not_found,expired}.json && diff /tmp/{accepted,revoked}.json
```
Expected: no diff output.

## 5. Self-invite rejected
- [ ] Owner attempts to invite their own email → 400 `self_invite`.

## 6. Cross-instance fan-out (skip if single-instance staging)
- [ ] Open editor in two browsers from different staging pods.
- [ ] Add an item in one → see it in the other within 5 s.

## 7. Downgrade → kick → upgrade → restore
- [ ] Editor connected to calendar editor.
- [ ] Trigger Stripe downgrade webhook.
- [ ] Editor: kick toast + redirect; calendar gone from "Shared with you".
- [ ] Re-upgrade owner.
- [ ] Editor: receives "access restored" email; calendar back in "Shared with you".

## 8. Mobile editor
- [ ] In mobile viewport, presence chip visible in header.
- [ ] Members tab opens in settings drawer; invite modal renders correctly.

## 9. Email rendering
- [ ] Real Gmail (light + dark theme) — calendar name renders without HTML escape sequences leaking.
- [ ] Real Outlook web — link is clickable.

## 10. Token scrubbing in logs
```bash
gcloud logging read "resource.type=cloud_run_revision AND textPayload:invite" --limit 100 --format json \
  | jq -r '.[].textPayload' | grep -E '\?token=[^&]+' | wc -l
```
Expected: `0`.
```

- [ ] **Step 2: Commit**

### Task 5.5 — Token-scrubbing deployment-plan addendum

**Files:**
- Modify: existing deployment plan / spec (`docs/superpowers/specs/2026-04-20-deployment-design.md` or successor)

- [ ] **Step 1: Append section**

```markdown
## Token scrubbing (added 2026-05-05 for spec 2)

Co-editor sharing introduces invitation tokens in URL query strings. Before launch, configure:

- **Cloudflare:** Transform Rule on the `/invitations/` path family — regex replace `(\?|&)token=[^&]+` → `$1token=REDACTED` in `Request URI` field of the access log.
- **Cloud Run:** Cloud Logging exclusion filter:
  ```
  resource.type=cloud_run_revision
  httpRequest.requestUrl=~"\\?token="
  ```
  → severity-DEFAULT exclusion. Alternative: sink-side regex redaction.
- **App-side:** confirm Actix `Logger::default()` format string excludes the query string. The default `%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T` includes the path-and-query in `%r`. Override to `%a "%m %P" %s %b ...` (method + path only) for sharing routes if needed.

Verification post-deploy: see UAT §10.
```

- [ ] **Step 2: Commit**

### Task 5.6 — Phase 5 verification gate

- [ ] **Run all tests both sides + E2E.**
- [ ] **Run Manual UAT** (or document as a release blocker for the assigned operator).
- [ ] **Update plan status table — all phases ✅.**
- [ ] **Final commit.**

---

## Self-review — checked

**Spec coverage:**
- §1 architecture → covered by all phases.
- §2 schema → Tasks 0.1, 0.2, 0.3, 1.5.5.
- §3 HTTP API surface → Tasks 0.11 (GET /calendars), 1.6 (sharing controller), 2.1 (per-item), 3.4 (SSE), 3.3 (presence heartbeat).
- §3.7 security mandates → Task 1.2 (rate limit), 1.3 (validation), 1.5 (anti-enumeration via single error shape), 1.5 (email-bound accept).
- §3.8 token-scrubbing → Task 5.5.
- §3.9 2FA → memory `reference_2fa_invitation_gate_backlog.md`; not in plan, intentional.
- §4 live sync → all of Phase 3.
- §5 tier transitions → Phase 4.
- §6 frontend surfaces → Tasks 0.11, 2.3, 3.6, 3.7, 4.4, 5.1.
- §7 testing → coverage in each task + Tasks 5.2, 5.3, 5.4.

**Placeholder scan:** no TBDs. The mid-plan migration for `token_lookup` (1.5.5) is intentional and follows naturally from realising in 1.5 that argon2 hashes can't be indexed.

**Type consistency:**
- `CalendarEditor` / `CalendarInvitation` / `InvitationStatus` / `Action` / `CalendarEvent` consistent throughout.
- `assert_can` signature stable across 0.8 / 0.9 / 1.6 / 2.1 / 3.4.
- `meta_version: i32` (server) / `metaVersion: number` (frontend) — stable.

---

## References

- Spec: `docs/superpowers/specs/2026-05-05-co-editor-sharing-design.md`.
- `feedback_audit_lock_in_tx_consistency.md` — `_in_tx` discipline for advisory-locked sections.
- `feedback_no_sqlx_test_use_test_pool.md` — testing convention.
- `feedback_actix_governor_path_exempt.md` — custom `KeyExtractor` pattern.
- `feedback_vue_router_mock_leaks_across_workers.md` — frontend test history.
- `feedback_uimodal_teleport_tests.md` — Teleport test pattern.
- `feedback_playwright_route_registration_order.md` — Playwright fixture ordering.
- `reference_402_reason_routing.md` — 402 pipeline this hooks into.
