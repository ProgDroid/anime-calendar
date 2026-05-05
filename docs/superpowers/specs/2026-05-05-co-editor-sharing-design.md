# Co-Editor Sharing — Design

**Date:** 2026-05-05
**Status:** Draft (brainstorm output, pre-implementation-plan)
**Spec ID:** spec-2 (sibling specs: spec-1 monetisation refinement; spec-3 deployment; spec-4 MAL list sync)
**Owner:** Fernando

## 1. Overview

Allow Pro users to share calendars with other accounts as **co-editors**. Editors can add and remove items on the shared calendar; only the owner can rename, recolour, change `event_style`, change reminders, delete, or invite/remove other editors. The feature is gated behind the Pro tier on the **owner side only** — an editor's own tier is never consulted for authorization on a shared calendar.

The feature is built around five core concerns:

1. **Membership model** — owner + up to 5 active editors per calendar; suspend (not delete) on owner downgrade.
2. **Mutation API** — split per-item endpoints (`POST /items`, `DELETE /items/:media_id`) for editors; meta endpoints stay owner-only.
3. **Authorization** — single `assert_can(actor, calendar, action)` helper; tier check folded into `Action::Invite` only.
4. **Live sync** — Server-Sent Events (SSE) over Upstash Redis Pub/Sub for cross-instance fan-out; presence model with 60 s TTL + 30 s heartbeat.
5. **Notifications** — transactional email at invitation send; SSE-driven toasts for member.* and kick events; no item.* toasts.

## 2. Schema changes

Three additions, all reversible:

### 2.1 `calendars.meta_version`

```sql
ALTER TABLE calendars ADD COLUMN meta_version INTEGER NOT NULL DEFAULT 1;
```

Bumped exactly once per `PUT /calendars/:id` that changes any meta field (name, accent, event_style, reminder defaults). Used by SSE consumers to drop stale frames during reconnect-replay races. Item.* events also carry the post-write `meta_version` for ordering.

### 2.2 `calendar_editors`

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
```

- Composite PK — no surrogate key (relationship is the identity; nothing else points at it).
- `active = false` + `suspended_at` set means "preserved during owner downgrade for restore-on-upgrade." Hard delete only on explicit owner-revoke or editor-leave.

### 2.3 `calendar_invitations`

```sql
CREATE TABLE calendar_invitations (
    id             BIGSERIAL PRIMARY KEY,
    calendar_id    BIGINT       NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    inviter_id     BIGINT       NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
    invitee_email  CITEXT       NOT NULL,
    token_hash     TEXT         NOT NULL,        -- argon2id of the random 256-bit token
    status         TEXT         NOT NULL DEFAULT 'pending',
                              -- pending | accepted | declined | revoked | expired | suspended
    expires_at     TIMESTAMP    NOT NULL,        -- now() + 7 days
    sent_at        TIMESTAMP    NOT NULL DEFAULT NOW(),
    resolved_at    TIMESTAMP    NULL
);

CREATE UNIQUE INDEX idx_calendar_invitations_pending_unique
  ON calendar_invitations (calendar_id, lower(invitee_email))
  WHERE status = 'pending';
```

- `token_hash` is argon2id of the raw token. Hash-not-store pattern; the raw token only ever exists in transit (URL + email body).
- Partial unique index forbids two pending invites for the same `(calendar, email)` — a fresh invite to the same person requires the prior one to be resolved.
- `status='suspended'` reserved for owner-downgrade preservation (see §5.1).
- 7-day expiry; configurable via env if needed later.

## 3. HTTP API surface

### 3.1 Mutations (split shape)

| Verb | Path | Auth | Notes |
|---|---|---|---|
| `POST` | `/calendars/:id/items` | owner OR editor | Body: `{ media_id }`. Idempotent — returns 200 with `affected: false` if already present. Bumps no meta_version. |
| `DELETE` | `/calendars/:id/items/:media_id` | owner OR editor | Idempotent — 200 with `affected: false` if missing. |
| `PUT` | `/calendars/:id` | owner only | Meta-only (name, accent, event_style, default reminders). Bumps `meta_version`. |
| `DELETE` | `/calendars/:id` | owner only | Cascade deletes editors + pending invites. |

The existing batch endpoint shape is retired in favour of the per-item endpoints (UI no longer needs batch since the multi-select queue was removed in Track 2/3).

### 3.2 Member management

| Verb | Path | Auth | Notes |
|---|---|---|---|
| `GET` | `/calendars/:id/editors` | owner OR editor | Returns active editors + (owner-only) pending invites. |
| `DELETE` | `/calendars/:id/editors/:user_id` | owner only (not self) | Hard delete; emits `member.left` + `kick`. |
| `DELETE` | `/calendars/:id/editors/me` | editor only | Editor leaves voluntarily. |
| `POST` | `/calendars/:id/invitations` | owner only, **Pro only** (402 if Free) | Body: `{ email }`. Rate-limited 20/hr per inviter. |
| `DELETE` | `/calendars/:id/invitations/:id` | owner only | Revoke pending invite. |
| `POST` | `/calendars/:id/invitations/:id/resend` | owner only | Bumps `expires_at = now() + 7d`, re-sends email. Same rate limit. |
| `GET` | `/invitations/:token` | public | Preview: calendar name, owner display, item count. Anti-enumeration: 400 same shape on any failure. |
| `POST` | `/invitations/:token/accept` | authed, email-bound | 403 if `current_user.email != invitation.invitee_email`. |
| `POST` | `/invitations/:token/decline` | authed, email-bound | Same email check. |

### 3.3 Calendar list

`GET /calendars` returns:

```json
{
  "owned":           [ { ...calendar, editor_count } ],
  "shared_with_me":  [ { ...calendar, owner: { id, display, avatar } } ]
}
```

Free user with shared calendars: `owned` enforces 3-cap; `shared_with_me` is unlimited and does not count toward the cap (per product decision — sharing is owner-funded, taking up free recipient's cap would be hostile).

### 3.4 Live sync

| Verb | Path | Auth | Notes |
|---|---|---|---|
| `GET` | `/calendars/:id/events` | owner OR editor | SSE stream. See §4. |
| `POST` | `/calendars/:id/presence/heartbeat` | owner OR editor | 30 s cadence; returns 204. |

### 3.5 Error contract

All error responses use existing `Error` JSON shape. New variants:

- `Error::Conflict { reason: "editor_cap_reached" }` → 409
- `Error::PaymentRequired { reason: "share_calendar" }` → 402 (existing pipeline; new reason key)
- `400 { "error": "invalid_invite_token" }` — single shape for all token failure modes (anti-enumeration)

### 3.6 Authorization helper

```rust
enum Action { ItemMutate, MetaMutate, ManageEditors, Invite }

async fn assert_can(actor: &Claims, cal: &Calendar, action: Action) -> ServerResult<()> {
    match action {
        Action::ItemMutate     => if actor.id == cal.owner_id || is_active_editor(actor.id, cal.id).await? { Ok(()) } else { Err(Forbidden) },
        Action::MetaMutate     => if actor.id == cal.owner_id { Ok(()) } else { Err(Forbidden) },
        Action::ManageEditors  => if actor.id == cal.owner_id { Ok(()) } else { Err(Forbidden) },
        Action::Invite         => {
            if actor.id != cal.owner_id { return Err(Forbidden); }
            if !subscription::is_paid(actor.id).await? { return Err(PaymentRequired { reason: "share_calendar" }); }
            Ok(())
        }
    }
}
```

### 3.7 Security requirements (mandatory)

1. **Anti-enumeration** on `GET/POST /invitations/:token` — every token failure mode (not found, expired, accepted, declined, revoked, suspended) returns identical `400 { "error": "invalid_invite_token" }`.
2. **Per-inviter rate limit** — 20 invite-sends per hour keyed on `actor_user_id` via custom `actix-governor` `KeyExtractor` (per `feedback_actix_governor_path_exempt`).
3. **Send-time validation** — RFC 5321 email format, 254-char max, reject self-invite (`invitee_email.lower() == owner.email.lower()`), HTML-escape calendar name and owner display in email templates.
4. **Email-bound accept/decline** — `current_user.email.lower() == invitation.invitee_email.lower()` or 403.
5. **CSRF** — covered by existing SameSite=Lax cookie + body-token pattern; no new machinery.

### 3.8 Operational note (deployment plan addendum)

Tokens appear in URL query strings and email bodies. Scrub `?token=` query strings from:

- Cloudflare access logs (transform rule: regex-replace `(\?|&)token=[^&]+` → `$1token=REDACTED`).
- Cloud Run request logs (Cloud Logging exclusion filter on `/invitations/` paths or sink-side regex redaction).
- `RUST_LOG=info` access traces (Actix Logger format string excludes query string by default, verify).

Add to deployment-plan checklist as an explicit pre-launch verification step.

### 3.9 2FA gate (deferred)

When 2FA lands as its own auth-hardening spec, layer fresh-2FA-challenge requirements on `POST /calendars/:id/invitations` and `POST /invitations/:token/accept` for users who have 2FA enabled. Not in scope for this spec. Tracked in memory `reference_2fa_invitation_gate_backlog.md`.

## 4. Live sync (SSE + Pub/Sub)

### 4.1 Event taxonomy

Single per-calendar stream. Each frame is a discriminated JSON envelope:

```json
{ "type": "item.added",   "media_id": 12345, "actor": "u_42", "v": 18, "at": "..." }
{ "type": "item.removed", "media_id": 67890, "actor": "u_42", "v": 19, "at": "..." }
{ "type": "meta.updated", "fields": ["name","accent"], "actor": "u_42", "v": 20, "at": "..." }
{ "type": "member.joined","user_id": "u_99", "display": "Alex", "actor": "u_42" }
{ "type": "member.left",  "user_id": "u_99", "actor": "u_42", "reason": "removed|left|downgrade" }
{ "type": "presence",     "viewers": [ {"user_id":"u_42","display":"Nando"}, ... ] }
{ "type": "kick",         "reason": "role_revoked|owner_downgrade|calendar_deleted" }
```

`v` is the post-write `meta_version`. Clients drop frames where `v <= local.meta_version`. `actor = "self"` when echoed back to the originator (suppresses double-toast).

### 4.2 Pub/Sub channels (Upstash Redis)

- `cal:<id>` — every member's SSE handler subscribes; receives `item.*`, `meta.*`, `member.*`.
- `presence:<id>` — separate to isolate heartbeat throughput.
- `kick:<user_id>` — per-user; handler subscribes to its own; tears down all active streams for that user on receipt.

### 4.3 Presence

Redis keys, 60 s TTL, refreshed by 30 s client heartbeat:

- `presence:cal:<id>:user:<uid>` → `{ "display": "...", "since": "..." }`
- Heartbeat = `POST /presence/heartbeat` → server `SET ... EX 60 XX` (refresh-only) → `PUBLISH presence:<id>`.
- Viewer list = `SCAN presence:cal:<id>:user:*` on first SSE attach + on every presence event.
- Multi-tab from one user → deduped by user_id (display once).
- Presence events carry the full viewer list, not deltas (lists are tiny: ≤ 6 users).

### 4.4 Kick semantics

When an editor's role is revoked (owner kick / owner downgrade / calendar delete):

1. Server transaction marks `calendar_editors.active = false` (or hard-delete on explicit revoke), removes pending presence row, commits.
2. Server `PUBLISH kick:<user_id>` with reason.
3. Every Actix instance hosting an SSE stream for that user closes the response.
4. Client receives the `kick` frame just before the stream closes; UI shows toast and routes to `/my-calendars`.

### 4.5 Reconnect contract

- Heartbeat comment frame (`: ping\n\n`) every 25 s to keep idle proxies open (Cloudflare 100 s timeout).
- On client reconnect (auto via EventSource), server re-emits current `meta_version` and viewer list as the first two frames; client reconciles without a full GET.
- No `Last-Event-ID` replay in v1; clients trust the reconcile snapshot.

### 4.6 Event origination boundary

Each mutation handler emits exactly one event **after** its DB transaction commits. Failure to publish does not roll back the write — readers get the change on next reconnect.

| Handler | Emits |
|---|---|
| `POST /items` | `item.added` (skip on idempotent re-add) |
| `DELETE /items/:media_id` | `item.removed` |
| `PUT /calendars/:id` (meta) | `meta.updated` |
| `POST /invitations/:token/accept` | `member.joined` |
| `DELETE /editors/:user_id` | `member.left` (reason: removed) + `kick` |
| Stripe webhook → downgrade reconcile | `member.left` (reason: downgrade) + `kick` per affected editor |

### 4.7 Cost / deployment notes

- Cloud Run min-instances stays at 0. Calendar load HTTP request warms the instance before the SSE connection is initiated; cold-boot is paid by the page load, not the stream.
- Upstash Redis Pub/Sub commands at expected v1 scale (≤ 100 concurrent shared-calendar viewers) cost ~$1-2/month above existing cache spend.

## 5. Pro tier transitions

### 5.1 Owner downgrade to Free

Single transaction:

1. `subscriptions.tier = 'free'`.
2. For each owned calendar with active editors: `UPDATE calendar_editors SET active=false, suspended_at=NOW() WHERE active=true`.
3. For each owned pending invite: `UPDATE calendar_invitations SET status='suspended'`.
4. Refresh frozen-blob subscribe ICS for affected calendars (existing Phase 1 path).
5. Commit.
6. After commit: `PUBLISH kick:<editor_id>` per affected editor (reason: `owner_downgrade`); `PUBLISH cal:<id>` `member.left` for editors viewing the same calendar.

Owner is **not** kicked from anything — they keep edit access to their own calendars. The "+ Invite editor" button is replaced with a Pro-locked CTA that opens the existing 402 modal with reason `share_calendar`.

### 5.2 Owner re-upgrade to Pro

Single transaction:

1. `subscriptions.tier = 'paid'`.
2. `UPDATE calendar_editors SET active=true, suspended_at=NULL WHERE suspended_at IS NOT NULL` for owned calendars (skips editors whose accounts no longer exist via FK).
3. `UPDATE calendar_invitations SET status='pending' WHERE status='suspended' AND expires_at > NOW()`. Expired ones stay suspended; owner re-invites manually.
4. Drop frozen-blob subscribe ICS (set NULL → live mode resumes).
5. Commit.
6. After commit: send transactional email "Your editor access to <calendar> has been restored" per restored editor.

**Restore window: unbounded.** Suspended editors are restored whenever the owner returns to Pro, regardless of how long ago the downgrade was. Matches the accent-preservation precedent in `feedback_tier_gated_apply_pattern`.

### 5.3 Editor's tier — irrelevant

Editor permissions are owner-funded. We never read the editor's tier when authorizing item.* operations. Free users can edit a Pro-owner's calendar without limit; the calendar's caps are owner-owned (unlimited shows, etc., flow from owner's tier).

### 5.4 Five-editor cap

Hard limit at 5 active-or-pending editors per calendar (anti-abuse, not monetisation). Enforced inside an advisory-locked transaction:

```rust
pg_advisory_xact_lock(hashtext(format!("invite:{calendar_id}")));
let active  = count_active_editors_in_tx(&mut *tx, calendar_id).await?;
let pending = count_pending_invites_in_tx(&mut *tx, calendar_id).await?;
if active + pending >= 5 {
    return Err(Error::Conflict { reason: "editor_cap_reached" });
}
```

Pending invites count toward the cap so an owner can't mass-blast 50 invites and have them all flip active. Decline / expire / revoke decrement.

No 402 here — modal copy: "This calendar already has 5 editors (active or invited). Remove someone or wait for an invite to expire."

### 5.5 New 402 reason: `share_calendar`

Added to existing 402 routing pipeline (`reference_402_reason_routing`). Locale keys:

- `interrupt.heading.share_calendar` — "Sharing calendars is a Pro feature"
- `interrupt.description.share_calendar` — body copy
- CTA → `/upgrade`

### 5.6 UpgradePage feature list

Replace existing "Early access to new features" bullet with:

> **Share calendars with co-editors** (up to 5 per calendar)

Final 6-feature list:

1. Unlimited calendars
2. Unlimited tracked shows
3. Live subscribe URL with hourly refresh
4. Customisable reminders (up to 5)
5. All accent themes — Coral, Iris, Matcha, Sakura, Citron
6. Share calendars with co-editors (up to 5 per calendar)

`upgrade.proFeatures.earlyAccess` is retired; mirror retirement in `pt.json`.

### 5.7 Owner account deletion

Existing `ON DELETE CASCADE` on `calendar_editors.calendar_id` and `calendar_invitations.calendar_id` covers cleanup. Before commit of the user-delete transaction, `PUBLISH kick:*` per editor with reason `calendar_deleted` so streams close cleanly.

## 6. Frontend surfaces

### 6.1 `MyCalendarsPage.vue` — split sections

Two-section layout: `My calendars` (owned, with cap counter) + `Shared with you` (rendered only if non-empty).

- Shared cards: "Owner: <display>" subtitle, **no delete button**, "Leave" action in card menu (`DELETE /editors/me`).
- Owned cards: `👤 +N` chip when calendar has active editors; click navigates to Members tab.
- Cap counter (3 / 3) only counts owned. Shared calendars never count toward Free cap.

### 6.2 `CalendarSettingsForm.vue` — Members tab (owner-only)

New tab in the existing settings tab strip (between general settings and danger zone). Visible only when `current_user.id === calendar.owner_id`.

- **Active editors** list — avatar, display name, "Remove" button.
- **Pending invites** list — invitee email, age, expiry countdown, "Resend" + "Revoke" buttons.
- **+ Invite editor** modal — single email input → `POST /invitations`.
- For Free owners: entire tab is replaced with `<UpgradeLockOverlay>` (same component the reminders section uses); CTA opens 402 modal with `share_calendar` reason.

### 6.3 Presence chip in editor header

`<PresenceChip />` in `CalendarEditorViewDesktop.vue` and `CalendarEditorViewMobile.vue` near the title row.

- Shows "👁 You + N viewing" with click-to-expand popover listing all current viewers (avatar + display).
- Backed by `usePresence(calendarId)` composable that owns the SSE connection + heartbeat interval.
- The same composable owns live-mutation event handlers — single EventSource per page.
- Disconnects on `onBeforeUnmount` and route change.

### 6.4 `/invite/:token` landing page

Public route (`meta: { public: true }`).

- **Unauthenticated**: preview card (calendar name, owner display, owner avatar, item count) + "Sign in to accept" / "Don't have an account? Sign up" buttons. Both CTAs include `?redirect=/invite/<token>` so post-auth they bounce back.
- **Authenticated, email mismatch**: "This invite is for <masked-email>. Sign in with that account to accept." (no Accept button).
- **Authenticated, email match**: Accept + Decline buttons. Accept → `POST /invitations/:token/accept` → redirects to `/calendars/:id`. Decline → `POST .../decline` → returns to `/my-calendars` with toast.
- **Token failure**: identical generic page for all failure modes — "This invitation link is no longer valid. Ask the calendar owner to send a new one." (anti-enumeration).

### 6.5 Toast catalogue (i18n keys under `sharing.toasts.*`)

SSE-driven toasts:

- `member.joined` (other editors): "<name> joined this calendar"
- `member.left` reason=removed: "<name> was removed by the owner"
- `member.left` reason=left: "<name> left this calendar"
- `kick` reason=role_revoked: "Your access to <calendar> has been removed"
- `kick` reason=owner_downgrade: "<owner> downgraded to Free, so editor access is suspended. You'll be restored if they upgrade again."
- `kick` reason=calendar_deleted: "<calendar> was deleted by the owner"

`item.added` and `item.removed` do **not** toast — the in-memory list updates silently. Toasting every add would be noisy during co-editor bulk searches.

### 6.6 Editor — collision feedback

`meta.updated` event with `v` ahead of local pending edit:

- If form is dirty: inline banner above the form: "Settings were just updated by <actor>. Refresh to see changes — your unsaved edits will be kept locally."
- Refresh button reloads server state into the form's "original" baseline; user's dirty fields untouched.
- Item events during dirty meta edit: just update the items list silently; meta form stays dirty.

### 6.7 Locale keys

New top-level namespace `sharing` (greppable; easy to retire). ~30 keys total: members tab, invite modal, landing page, toasts, presence labels, error states. Mirror in both `en.json` and `pt.json` per project convention.

## 7. Testing strategy

### 7.1 Server unit + integration (Rust)

Use existing `test_pool()` / `test_tx()` (per `feedback_no_sqlx_test_use_test_pool`); never `#[sqlx::test]`.

**Mapper-level (`test_tx`):**
- Composite-PK insert/upsert idempotency on `calendar_editors`.
- Partial unique index forbids two pending invites for same `(calendar, email)`; allows new pending after revoke.
- `_in_tx` variants used inside advisory-locked write paths (per `feedback_audit_lock_in_tx_consistency`).
- `meta_version` increments exactly once per `PUT` with changed fields; no-op `PUT` does not increment.

**Service-level (`test_pool`):**
- Cap enforcement: 5-active-or-pending boundary returns `Conflict { reason: "editor_cap_reached" }`.
- Anti-enumeration: 4+ token failure modes return byte-equal `400 invalid_invite_token`.
- Email-bound accept: actor mismatch returns 403; exact match succeeds.
- Self-invite rejected at validation.
- Per-inviter rate limit: 21st invite/hour returns 429.
- Argon2id roundtrip: mint → hash → verify; tampered token rejected.
- Suspend-on-downgrade transaction: row-count assertions before/after.
- Restore-on-upgrade: suspended → active; expired suspended invites stay suspended; meta_version not bumped.

**HTTP-level (controllers):**
- Editor returns 403 on `POST /invitations`; owner succeeds.
- Editor can `POST /items` and `DELETE /items/:media_id`; cannot `PUT /calendars/:id`.
- Free owner clicking invite returns `402 { reason: "share_calendar" }`.
- `GET /calendars` response shape: `{ owned, shared_with_me }`.

### 7.2 SSE + Pub/Sub integration

Containerised Redis (Upstash-protocol-compatible) for CI.

**Single-instance:**
- Spawn Actix on bound port, open EventSource client; assert mutation → frame within 500 ms with correct discriminator + `v`.
- Heartbeat comment frame at expected cadence.
- Kick publish closes stream within 500 ms.

**Cross-instance (the fan-out test):**
- Two Actix instances, same Redis. Connect SSE to A, mutate via B. Assert A's client receives the frame.
- One per event family is enough — channel routing is shared.

### 7.3 Frontend unit (Vitest)

Per project conventions:
- `createMemoryHistory` always; never `vi.mock('vue-router')` (per `feedback_vue_router_mock_leaks_across_workers`).
- `attachTo: document.body` for `<UiModal>` content assertions (per `feedback_uimodal_teleport_tests`).
- `data-testid` selectors over class selectors.

**Coverage:**
- `MyCalendarsPage` — `Shared with you` only when non-empty; cap counter excludes shared; "Leave" wired correctly.
- `CalendarSettingsForm` Members tab — owner sees full UI; editor doesn't see tab; Free owner sees `<UpgradeLockOverlay>`.
- Invite modal — RFC 5321 client-side validation; submit → store action; success closes modal.
- `/invite/:token` page — four parameterised render states.
- Presence chip — popover open/close, viewer list.
- `usePresence` — opens EventSource on mount; closes on unmount; dispatches frames; handles reconnect.

**Mocking:**
- Global `EventSource` mock in `vitest.setup.ts` with `mockSse.push({...})` for synchronous frame injection.
- Per-test axios stubs; no MSW.

### 7.4 E2E (Playwright)

Per `feedback_playwright_route_registration_order` (catch-all `**/api/**` first, specific stubs last) and `feedback_playwright_body_locator_fragility`.

**Scenario 1: Happy-path invite + accept**
- Owner (storage state A) opens calendar → Members tab → invites editor@test.
- New context (storage state B = editor@test) opens email link → preview → accept.
- Editor lands on calendar editor.
- Owner's tab sees `member.joined` toast and presence chip update.
- Editor adds item; owner's tab sees the row appear without manual refresh.

**Scenario 2: Owner downgrade kicks editor**
- Both tabs in editor and connected.
- Trigger downgrade via test-only endpoint (cfg(test) gate, same pattern as Track 4).
- Editor's tab gets kick toast + redirect to `/my-calendars`.
- Editor's `/my-calendars` no longer shows the calendar in `Shared with you`.
- Owner's tab shows `member.left` reason=downgrade in members list.

Total target: 2-3 E2E specs. More granular cases live in component tests.

### 7.5 Manual UAT

`docs/checklists/2026-05-co-editor-sharing-uat.md` written alongside this spec. Sections:

1. Invite happy path (real email)
2. Invite expiry
3. Cap enforcement (5 active, 6th blocked)
4. Anti-enumeration (curl 4 token failure modes, byte-compare responses)
5. Self-invite rejected
6. Cross-instance fan-out (two browsers on different staging instances if available)
7. Downgrade → kick → re-upgrade → restore (full lifecycle)
8. Mobile editor — presence chip, members tab in drawer
9. Email rendering (real Gmail/Outlook clients, dark + light)
10. Token-scrubbing in access logs (grep for `?token=`, expect zero hits)

### 7.6 Load / soak (out of scope for v1)

Real load testing of SSE concurrency parked until paying users exist. Note: revisit when crossing 100 concurrent shared-calendar viewers. Five-editor cap limits worst-case fan-out to small numbers anyway.

## 8. Open items deferred to implementation plan

- Exact migration sequencing (migrations 0XXX–0XXY).
- Email template HTML (transactional template lives in code; copy approved separately).
- Concrete `actix-governor` `KeyExtractor` implementation for per-user rate limiting.
- Concrete SSE handler shape (likely `actix-web-lab::sse`).
- Whether to expose `meta_version` in `GET /calendars/:id` response or leave it strictly internal.

These are filled in during writing-plans, not here.

## 9. References

- `reference_402_reason_routing.md` — existing 402 pipeline this hooks into.
- `reference_audit_trails_backlog.md` — unified audit subsystem deferred; do not bake invitation log into it ad-hoc.
- `reference_2fa_invitation_gate_backlog.md` — 2FA gate to layer on once 2FA spec ships.
- `feedback_tier_gated_apply_pattern.md` — preserve-stored-on-downgrade precedent that suspend-on-downgrade follows.
- `feedback_audit_lock_in_tx_consistency.md` — `_in_tx` discipline for advisory-locked write paths.
- `feedback_actix_governor_path_exempt.md` — custom `KeyExtractor` pattern for per-user rate limiting.
- `feedback_no_sqlx_test_use_test_pool.md` — testing convention; avoid `#[sqlx::test]`.
- `feedback_vue_router_mock_leaks_across_workers.md` — frontend test convention.
- `feedback_playwright_route_registration_order.md` — Playwright fixture ordering.
