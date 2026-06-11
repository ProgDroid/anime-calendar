# anime-calendar — Code Audit

**Date:** 2026-05-07
**Scope:** Backend (Rust workspace), Frontend (Vue SPA), Infra & boundaries (Docker/nginx/CI), uncommitted privacy/terms work
**Method:** 6 specialist subagents in parallel — backend security, backend quality, Stripe/billing, co-editor sharing + SSE, frontend security/quality, infra/boundaries — each seeded with project memory so they would not re-flag settled ground.

---

## Executive summary

The codebase is in materially good shape: the documented conventions (mapper layering, anti-enumeration, advisory locks, SSE actor filtering, httpOnly cookies, CSP discipline at the API layer) are mostly followed; the few known gaps from memory have been resolved. The findings below are concentrated at three places:

1. **Boundary failures between subsystems**, where one subsystem's contract has drifted from another's expectation. The two CRITICALs are exactly this shape.
2. **Drift in places where a memory-encoded rule already exists** but a single edit slipped through (anti-enumeration `400 vs 403` in invitation flows; `_with` vs `_in_tx` mapper suffix; `vi.mock('vue-router')` in one test file).
3. **Infra/perimeter** — nginx security headers, SSE proxy buffering, default-bound DB/Redis ports, missing CSP — all addressable in one config-only pass.

No live data-loss or auth-bypass bugs were found. Two billing/sharing CRITICALs and several injection-class HIGHs warrant fixing before any expanded public release.

---

## Severity overview

| Severity | Count | Where |
|---|---|---|
| CRITICAL | 2 | Billing (account-delete abandons sub), Sharing (SSE actor filter mismatch) |
| HIGH | 18 | Backend security (3), Sharing (3), Billing (2), Frontend (3), Infra (4), Backend quality (3) |
| MEDIUM | 30 | Spread across all domains |
| LOW / NIT | 40+ | Style, dead code, logging, doc gaps |

---

## CRITICAL findings

### C-1. Account deletion silently abandons live Stripe subscriptions
**File:** `server/src/controllers/user.rs:255`, `server/src/mappers/user.rs:231`
`delete_user` soft-deletes the user and cascades calendars only — no Stripe `cancel_subscription` API call, no local subscription row update, no co-editor cleanup. The user keeps being billed forever; their `stripe_customer_id` is orphaned; co-editors retain access to soft-deleted calendars until owner-side filters kick in.
**Fix (atomic, single PR):**
- In `delete_user_in_tx`, fetch active sub via `subscription_mapper.find_active_for_user_in_tx(&mut *tx, user_id)`.
- Call Stripe `cancel_subscription` (or `update` with `cancel_at_period_end=true`).
- Call `services/sharing::suspend_owner_sharing_in_tx(&mut *tx, user_id)`.
- Update local sub row to `canceled`.
- Invalidate refresh tokens in the same tx.
- Acceptance: deleting a user with an active sub leaves no live Stripe subscription and no live editor rows.

### C-2. SSE `ItemAdded` / `ItemRemoved` actor filter is broken — users see their own actions echoed
**Files:** `server/src/controllers/calendar.rs:1046, 1109` (publishes `actor: actor_id.to_string()`, e.g. `"42"`); `frontend/src/stores/auth.ts:9` (`authStore.user` is the **username** from `response.data.username`); `frontend/src/components/calendar/CalendarEditorViewDesktop.vue:322` (filters with `frame.actor !== authStore.user`).
The two strings will never match. Memory entry `feedback_sse_event_actor_filter` already documents the rule; the implementation contract drifted. Note `MetaUpdated` correctly uses `user.username` (`controllers/calendar.rs:620`) — only the item events are wrong.
**Fix:**
- Pick a canonical actor key. Recommended: `actor_id` (numeric, stable, never changes on rename).
- Standardize backend: every `Frame { actor: ... }` uses `actor_id.to_string()`.
- Standardize frontend: store `userId: number | null` in `auth.ts` alongside `user`. Filter against `frame.actor === String(authStore.userId)`.
- Add a smoke test: optimistically dispatch an item-add, fixture-publish the matching event, assert `itemsInCalendar` length unchanged.

---

## HIGH findings

### Backend security

**H-1. CRLF injection into iCalendar export via calendar `name`**
`server/src/services/ics_export.rs:167-170` (`empty_calendar_ics`) — `format!("...NAME:{name}...\r\n", name=&entity.name)` interpolates user-controlled `calendars.name` directly. A name containing `\r\n` injects arbitrary ICS lines (fake `BEGIN:VEVENT` blocks visible in any subscribed calendar app). `validate_name` (`controllers/calendar.rs:252-267`) only checks length. **Fix:** strip/replace CR/LF in `empty_calendar_ics` AND in the `validate_name` allow-list.

**H-2. `Content-Disposition` filename header injection**
`server/src/controllers/calendar.rs:327-330, 341-343` — `filename` is interpolated unquoted-escaped into `attachment; filename="{filename}"`. A name containing `"` or `\r\n` breaks out of the header. **Fix:** ASCII-strip + replace double-quote, or use `filename*` with RFC 5987 percent-encoding.

**H-3. Advisory-lock bypass on `POST /calendars/{id}/items` (`add_item`)**
`server/src/controllers/calendar.rs:1031-1035` — `entitlement.assert_can_add_show` is called pool-bound, then `calendars.add_item_idempotent` mutates without an outer `pg_advisory_xact_lock`. Two concurrent POSTs from the same Free user can both pass the cap check and exceed `free_show_cap`. Same pattern as `feedback_audit_lock_in_tx_consistency`. **Fix:** wrap in `pool.begin()` + `pg_advisory_xact_lock(user.id)` + `assert_can_add_show_in_tx` + `add_item_idempotent_in_tx`.

### Co-editor sharing

**H-4. Invitation tokens leak into nginx and actix access logs**
`frontend/nginx.conf` has no `access_log` override; the `safe_url` regex in `server/src/server.rs:201` only matches `/calendars/subscribe/`, not `/invitations/`. Tokens land in container stdout. The deployment doc references "token-scrubbing" but no code/config wires it up. **Fix:** extend `safe_url` regex to redact `/invitations/[^/]+`; add an nginx redact rule (or `access_log off;`) for the `/invite/*` SPA path.

**H-5. `accept` / `decline` leak token validity via 400 vs 403**
`server/src/services/invitation_service.rs:265-289` — invalid token → `Error::InvalidRequest` (400); valid token but wrong email → `Error::Forbidden` (403). The module-level docs explicitly promise all token failures collapse to 400. **Fix:** change the email-mismatch branch to `Err(Error::InvalidRequest)`. Add a property test: `Err(_)` on token-path always maps to 400.

**H-6. No SSE connection cap per user — trivial DoS**
`controllers/sse.rs::calendar_events` accepts unlimited concurrent EventSource subscriptions per user; each holds 3 broadcast receivers and a tokio task slot. **Fix:** per-user counter (Redis or in-memory map) capped at e.g. 8 concurrent connections.

### Stripe billing

**H-7. Reconcile-loop status regression possible**
`server/src/mappers/subscription.rs:367` — guard `WHERE id=$1 AND current_period_end <= $3` protects period-end regression but not status regression. If a webhook lands `active` and a subsequent reconcile pass reads stale Stripe truth as `past_due` with equal period boundaries, the row regresses. **Fix:** add `OR status <> $2` to the WHERE clause and re-evaluate the conditional.

**H-8. Webhook does not handle `customer.subscription.paused`, `customer.deleted`, `invoice.payment_action_required`**
`server/src/controllers/stripe_webhook.rs:330` — events fall through to no-op. Effects: `paused` → `Status::Paused` is correctly unreachable via `find_active_for_user`'s WHERE clause, but the row never updates locally; `customer.deleted` → local rows stale forever and reconcile soft-skips `RetrieveSubscription` 404s; SCA renewal → silent. **Fix:** add explicit handlers; for `customer.deleted`, flip status to `canceled` and call `suspend_owner_sharing_in_tx`.

### Frontend security

**H-9. Open redirect via protocol-relative URL**
`frontend/src/components/LoginPage.vue:44-45` — `redirect.startsWith('/')` accepts `//attacker.tld` (URL-parsed as protocol-relative). Currently flows only into `router.push` (relatively safe), but a future refactor to `window.location.href` would make this a phish vector. **Fix:** also reject `redirect.startsWith('//')` and `redirect.includes('\\')`. Stronger: validate via `new URL(redirect, location.origin).origin === location.origin`.

**H-10. Invite token leaks via `Referer` header**
`frontend/src/components/InviteLandingPage.vue:106, 113, 166` — invite token reproduced in `redirect=` query for `/login` and `/register`. Outbound nav (Google sign-in click) sends the raw token in `Referer`. **Fix:** store token in `sessionStorage`, redirect to `/login` without the query param. Add `<meta name="referrer" content="strict-origin">` defensively.

**H-11. `vi.mock('vue-router')` and `createWebHistory` in tests — directly violates documented rule**
`frontend/src/__tests__/auth.spec.ts:13` (vi.mock); `errorScenarios.spec.ts:14, 204`, `LoginPage.spec.ts:5, 17`, `ResetPasswordPage.spec.ts:5, 23` (createWebHistory). Memory entries `feedback_vue_router_mock_leaks_across_workers` and `feedback_test_history_pollution` already document the rule — these specs predate it. **Fix:** convert to `createMemoryHistory` + `router.push` spy.

### Infra

**H-12. No CSP / HSTS / security headers on frontend nginx**
`frontend/nginx.conf` ships zero security headers. The SPA's HTML/JS/CSS/fonts come straight from nginx. **Fix:** add `Content-Security-Policy`, `Strict-Transport-Security`, `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`, `Permissions-Policy`. Suggested CSP baseline:
```
default-src 'self';
img-src 'self' https: data:;
script-src 'self' 'sha256-<inline-theme-bootstrap-hash>' https://accounts.google.com https://js.stripe.com;
frame-src https://accounts.google.com https://js.stripe.com;
connect-src 'self' https://api.stripe.com;
font-src 'self';
```
The pre-paint inline script in `frontend/index.html:10-31` will need a hash (preferred over `'unsafe-inline'`).

**H-13. Backend `default-src 'none'` CSP applied to every API response**
`server/src/server.rs:216` — strict policy applied universally. Combined with H-12, the SPA gets *no* CSP and the API gets one strict enough to break in-browser embedded resources if anyone surfaces an API-served HTML page (e.g. Swagger UI when `enable_docs=true`). **Fix:** scope the strict CSP to API responses only (already true) and ensure the SPA gets its own (different) CSP via nginx. Document the layering.

**H-14. SSE proxy buffering will stall live-sync**
`frontend/nginx.conf:8-13` — no `proxy_buffering off`, no `proxy_read_timeout` override, no `Connection: ""`. Default `proxy_buffering on` + 60s `proxy_read_timeout` will stall `text/event-stream` frames; the 25s heartbeat usually saves it but any 60s+ idle window kills the connection. **Fix:**
```nginx
location /api/ {
  proxy_pass ...;
  proxy_buffering off;
  proxy_cache off;
  proxy_read_timeout 1h;
  proxy_set_header Connection "";
  chunked_transfer_encoding off;
}
```

**H-15. Postgres 5432 + Redis 6379 exposed on `0.0.0.0` in docker-compose**
`docker-compose.yml:11-12, 23-24` — default Postgres password `change-me`, no Redis AUTH. On a multi-tenant or LAN-exposed host, anyone can connect. **Fix:** bind to `127.0.0.1:` or drop the host port mapping for the default dev compose.

### Backend quality

**H-16. utoipa operationId collision risk on bare `get` handler**
`server/src/controllers/item.rs:19` — `pub async fn get(...)` has no explicit `operation_id`. Memory `feedback_utoipa_operation_id_collisions` says utoipa derives operationId from bare fn name and only redocly lint catches collisions. `controllers/items.rs::get` (line 33) and `controllers/public_config.rs::get` (line 28) have explicit ones; `item.rs::get` does not. **Fix:** add `operation_id = "get_item"`.

**H-17. Mapper convention drift — 7 files use `_with` instead of `_in_tx`**
`mappers/calendar.rs`, `mappers/email_verification.rs`, `mappers/password_reset.rs`, `mappers/refresh_token.rs`, `mappers/subscription.rs` (mixed), `mappers/user.rs`, `mappers/user_settings.rs` — all use `_with` suffix. Convention says `_in_tx`. Only `calendar_editor`, `calendar_invitation`, `stripe_event`, and the post-2026-05 half of `subscription` follow the rule. **Fix:** mass-rename. No LoC saved but eliminates a documented inconsistency that is now larger than the compliant surface.

**H-18. `services/email.rs` — 4 nearly-identical send fns (~150 LoC of copy-paste)**
Each of `send_password_reset` / `send_verification_email` / `send_invitation` / `send_editor_restored` re-parses `from`, parses `to`, builds the message, sets credentials, builds the mailer, sends. **Fix:** extract `send_plain_email(to, subject, body) -> ServerResult<()>`; refactor each to one-line builders. Adds testability — currently no tests in this module.

---

## MEDIUM findings (grouped)

### Security & cookies (backend)
- **M-1.** JWT validation does not pin algorithm or set required claims explicitly (`server/src/middleware/auth.rs:47`, `services/auth.rs:121`). `Validation::default()` defaults to HS256 in `jsonwebtoken 10`, but make the contract explicit: `validation.algorithms = vec![Algorithm::HS256]`, `validation.set_required_spec_claims(&["sub","exp","iss","aud"])`.
- **M-2.** Stripe API error string echoed to client (`server/src/controllers/stripe.rs:135, :222`). `Error::Stripe(e.to_string())` could leak price IDs / customer IDs / Stripe internals. **Fix:** log full error, return generic `"stripe error"`.
- **M-3.** `Error::EmailError(String)` leaks SMTP error message (`error.rs:42`). Same pattern.
- **M-4.** OAuth account-linking via Google ID token email (`controllers/oauth.rs:58`) can take over a local password account if the email matches. Document the trust boundary; consider requiring confirmation when first linking.
- **M-5.** Auth cookie `Path=/api/` (`controllers/auth.rs:80`) — correct today but invariant is undocumented; any future endpoint outside `/api/` won't receive the cookie.

### Sharing & SSE
- **M-6.** `cal_rx` / `presence_rx` Lagged silently dropped (`controllers/sse.rs:94, 98`). A slow client misses sync updates with no client-side hint. **Fix:** emit `{"type":"resync"}` on Lagged so the client refetches.
- **M-7.** `MembersTab.vue:38-43` swallows all errors silently for `removeEditor`/`revoke`/`resend`. **Fix:** surface a toast.
- **M-8.** `InviteLandingPage.vue:39` only catches 403; 400 (invalid/expired token) leaves state at `'ready'` with no feedback. **Fix:** branch on 400 → `state = 'invalid'`.

### Billing
- **M-9.** Reconcile loop is not multi-replica safe — no `pg_try_advisory_lock`. Two replicas double-bill drift counters and hammer Stripe API. **Fix:** wrap the loop in `pg_try_advisory_lock`.
- **M-10.** `LimitsConfig.free_calendar_limit` (3) and `free_show_cap` (25) duplicated as plain strings in `en.json` / `pt.json` (`"include 3 calendars"`, `"25 shows"`). Drifts silently. **Fix:** pull from runtime config endpoint or from entitlement response; use vue-i18n `<i18n-t>` named slots (memory: `feedback_i18n_t_named_slots`).
- **M-11.** Webhook handler does not fall back to `client_reference_id` if `metadata.user_id` is missing — Stripe-Dashboard-created subs without metadata become orphans (handled defensively only via customer-id lookup). Also: `client_reference_id` is set at Checkout (`stripe.rs:112`) but never read by the webhook. Either wire it through `checkout.session.completed` or remove it.
- **M-12.** `update_user_settings` accepts arbitrary `reminder_offsets_minutes` for Free users (preserves preference across upgrade/downgrade); only the .ics renderer enforces. If a future export surface bypasses `IcsExportService`, Free users get Pro reminders. **Fix:** document the invariant on `UserSettings`.

### Frontend
- **M-13.** No Content-Security-Policy on the SPA — see H-12 + H-13.
- **M-14.** Public-route prefix list in `frontend/src/config/api.ts:54-60` missing `/invite`, `/privacy`, `/terms`. Latent bug: if any of these ever calls a 401-returning endpoint (`InviteLandingPage` already does — `sharingService.preview(token)`), the axios interceptor will redirect to `/login` and lose the invite context. **Fix:** add the three prefixes.
- **M-15.** Dead `name` and `user_avatar` fields in `frontend/src/stores/auth.ts:9-11`. `ProfileTab.vue:31-32` reads from `localStorage` directly. **Fix:** remove the refs and the `localStorage.setItem` pair.
- **M-16.** `PrivacyPolicyPage.vue` and `TermsOfServicePage.vue` are near-identical copy/paste differing only in i18n keys. Extract a `LegalPagePlaceholder` component or wait until real content lands.
- **M-17.** `controllers/calendar.rs::put` is ~280 LoC; the file is 2,230 LoC. `controllers/stripe_webhook.rs` is 962 LoC. **Fix:** extract per-step helpers. (Doing this *before* fixing H-3 will make the lock-bypass fix cleaner.)

### Infra
- **M-18.** `config.docker.toml.dist` severely out of date — missing `[stripe]`, `[smtp]`, `[server.metrics]`, `[app]`, `[limits]`, `[sharing]`, `[cache]`. **Fix:** sync with `config.toml.dist`.
- **M-19.** Docker base image `rust:1.85-bookworm` is mid-2025 toolchain; ~6 months of CVE patches missing. **Fix:** bump.
- **M-20.** No `HEALTHCHECK` and no `tini`/`dumb-init` in backend Dockerfile. Signal forwarding from compose `stop` is fragile. **Fix:** add both.
- **M-21.** `npx --yes @redocly/cli@latest` in CI (line 241) pulls latest on every run — supply chain + non-reproducible. **Fix:** pin a version.
- **M-22.** No `cargo audit` / `cargo deny` / `npm audit` step in CI. **Fix:** add `cargo-deny` with a `deny.toml`.
- **M-23.** No `down.sql` for any of the 19 migrations — irreversible deploys. **Fix:** document the rollback strategy or add downs incrementally.
- **M-24.** No nginx-level rate limiting — verify `actix-governor`'s `KeyExtractor` actually reads `X-Forwarded-For` (`server.rs:170`). If it reads peer IP, rate-limiting collapses to "all of nginx is one client" in production.
- **M-25.** No startup assert that `cookie_secure = true` when `environment = "production"`. **Fix:** add a hard fail in `server.rs` startup.

### Test gaps
- **M-26.** No tests in `services/email.rs`, `services/calendar_events.rs`, `controllers/sharing.rs`, `controllers/stripe.rs`, `mappers/anilist.rs`. Per memory `project_track_4_phase_3_complete`, **webhook fixture tests were pending** — current state confirms no test exercises real signed payloads through `Webhook::construct_event`. Signature-verification regression would not be caught.
- **M-27.** No test for orphan-customer fallthrough path (`stripe_webhook.rs:416`).
- **M-28.** Missing scope-explicit operation in `redocly.yaml` lint — a new auth-required endpoint that forgets `#[utoipa::path(security(("..." = [])))]` won't be flagged. Add a project lint or comment.

### Backend code quality
- **M-29.** `services/sharing.rs` returns `Result<_, String>` instead of `ServerResult<_>` (lines 46, 119). **Fix:** convert.
- **M-30.** Calendar fetch SQL duplicated 4 times (`mappers/calendar.rs::get_calendar_by_id_with`, `get_calendar_by_token_with`, `get_by_id_any_owner_with`, `controllers/sharing.rs::load_calendar_any_owner`). **Fix:** consolidate; the controller helper is also a layering violation.

---

## LOW & NIT findings (selected — full lists in agent transcripts)

- L-1. `delete_user` does not invalidate refresh tokens (`mappers/user.rs:231-249`).
- L-2. `info!("{settings_data:?}")` (`controllers/user.rs:370`) — demote to debug.
- L-3. `info!("Processing item ...")` per-item logging in `services/ics_export.rs:194, 198` — high-volume; demote.
- L-4. JWT lacks `iat`/`nbf` claims (`services/auth.rs:80-91`).
- L-5. Three near-identical `Sha256 → hex` helpers (`services/auth.rs::hash_token`, `controllers/auth.rs::hash_refresh_token`, inline at `mappers/refresh_token.rs:212`).
- L-6. `futures = "0.3"` in `server/Cargo.toml:26` appears unused — only `futures_util` is imported.
- L-7. `controllers/user.rs:386-389` — 4 stale TODOs pointing at long-completed work.
- L-8. Cache key prefix versioning (`cache.rs::generate_item_meta_key/airing_key:297-303`) lacks `:v1` — adds bumping cost for the next field-add to `Item`.
- L-9. Signature webhook tolerance window — verify `async-stripe`'s default; pin explicitly.
- L-10. `useUpgradeInterrupt` reasons hardcoded — verify alignment with server's emitted set if a new gate ships.
- L-11. `usePresence.ts:43` heartbeat interval `30_000` ms hardcoded; backend's `presence_heartbeat_seconds` config not consulted.
- L-12. `AppFooter` may render under `UiBottomTabBar` on mobile auth routes.
- L-13. PrivacyPolicyPage / TermsOfServicePage back button uses bare `←` glyph — wrap with `aria-hidden="true"` and rely on the `t('app.back')` text for accessible name.
- L-14. `AppFooter` inner `<nav>` has no `aria-label` — multiple `<nav>` elements without distinct labels confuse screen readers.
- L-15. `console.error` calls in `main.ts:55`, `CalendarScheduleView.vue:45`, `useTheme.ts:66` — route through a single logger that can be disabled in prod.
- L-16. Grafana admin/admin in `ops/docker-compose.monitoring.yml:29-30` — fine for loopback dev; document.
- L-17. `bin/set_subscription.rs` uses `eprintln!` (lines 68, 285+) — convention says `log::error!`. CLI exemption is reasonable; document.
- L-18. `redocly.yaml` disables `security-defined` rule with rationale — operational risk noted in M-28.
- L-19. `LiveStripeFetcher::retrieve` (`reconcile.rs:97`) string-matches on `"No such subscription"` / `"resource_missing"` — brittle; pin via typed-error path.
- L-20. `subscribe_token` in `/calendars/subscribe/{token}` URL is in the path (redacted by `safe_url`) — verified safe.

(Approximately 20 more NIT-level items in the transcripts — formatting, docstring polish, comment rot — not enumerated here.)

---

## Privacy / Terms uncommitted work — review notes

The new `PrivacyPolicyPage.vue`, `TermsOfServicePage.vue`, `ui/AppFooter.vue`, plus the `App.vue` / `router/index.ts` / locales diffs are **safe to commit** with the following pre-merge fixes:

1. **Add `/privacy`, `/terms`, `/invite` to `PUBLIC_ROUTE_PREFIXES`** in `frontend/src/config/api.ts:54-60` (M-14).
2. **Wrap the back-button arrow** in `<span aria-hidden="true">` (L-13).
3. **Add `aria-label` to the inner `<nav>`** in `AppFooter.vue` (L-14).
4. Optional: extract a shared `LegalPagePlaceholder` component (M-16) or accept the duplication until real copy lands.
5. Verify both pages render correctly above `UiBottomTabBar` on mobile (L-12).
6. Confirm `docs/checklists/2026-05-legal-pre-release.md` gate before any non-placeholder content goes live.

i18n keys are symmetric across `en.json` and `pt.json` ✓.

---

## Recommended fix sequence

Severity is not the same as fix order. This sequence balances cost, blast radius, and dependencies between findings.

### Phase 1 — Cheap & critical (1–2 days)
- **C-2** SSE actor filter (1 backend + 1 frontend file, plus a smoke test). Unblocks correctness of every `ItemAdded`/`ItemRemoved` for editors.
- **H-1** ICS CRLF strip in `validate_name` and `empty_calendar_ics` (one shared helper).
- **H-2** `Content-Disposition` filename sanitize (one helper).
- **H-5** Collapse invitation `Forbidden` → `InvalidRequest` (one branch + one property test).
- **H-9** Reject `//` and `\` in `redirect=` query (one line).
- **H-11** Fix `vi.mock('vue-router')` and `createWebHistory` test files (mechanical).
- **M-14** Add `/privacy`, `/terms`, `/invite` to public-route prefix list.
- **M-25** Startup assert `cookie_secure=true` in production.
- **L-2 / L-3** Demote noisy log levels.

### Phase 2 — Cheap infra hardening (1 day, config-only)
- **H-12 / H-13** CSP + security headers in `frontend/nginx.conf`; verify backend CSP scope.
- **H-14** SSE-friendly `proxy_buffering off` + `proxy_read_timeout 1h` for `/api/`.
- **H-15** Bind Postgres/Redis to `127.0.0.1:` in `docker-compose.yml`.
- **M-21** Pin `redocly` version in CI.
- **M-22** Add `cargo-deny` step.
- **M-18** Sync `config.docker.toml.dist`.
- **M-19 / M-20** Bump Rust base image, add `HEALTHCHECK` + `tini`.

### Phase 3 — Critical with real surface (3–5 days)
- **C-1** Account deletion + Stripe cancel + sharing suspend (one transactional helper, multiple call-sites). Includes a fixture test that closes M-26 partially.
- **H-3** Advisory-lock bypass on `add_item`. Best done after **M-17** (extract `controllers/calendar.rs::put` and friends), so the lock wrap is a small diff in a smaller file.
- **H-7** Reconcile-loop status guard.
- **H-8** Webhook handlers for `paused`, `customer.deleted`, `invoice.payment_action_required`.
- **H-4** Token-scrubbing in nginx + `safe_url` regex.
- **H-6** SSE per-user connection cap.
- **H-10** Invite token via `sessionStorage` instead of `redirect=` query.

### Phase 4 — Quality & DRY (1–2 weeks, can parallelize with Phase 3)
- **H-16** utoipa `operation_id` on `item.rs::get`.
- **H-17** Mapper `_with` → `_in_tx` mass rename.
- **H-18** Extract `send_plain_email` helper in `services/email.rs`.
- **M-9** Reconcile loop multi-replica advisory lock.
- **M-17** Split `controllers/calendar.rs` and `stripe_webhook.rs`.
- **M-26 / M-27** Webhook fixture tests + sharing controller tests + email service tests.
- **M-29 / M-30** Convert `services/sharing.rs` to `ServerResult`; consolidate calendar fetch SQL.

### Phase 5 — Nice-to-haves
- All LOW / NIT items.
- M-23 (down migrations).
- M-2 / M-3 (error string redaction).
- L-19 (typed Stripe error matching).

---

## Coverage notes

### Files audited deeply
**Backend:** `server/src/server.rs`, `error.rs`, `middleware/auth.rs`, `services/auth.rs`, controllers in full (`auth`, `refresh`, `oauth`, `password_reset`, `email_verification`, `sse`, `calendar`, `sharing`, `stripe`, `stripe_webhook`, `items`, `user`), `services/ics_export.rs`, `services/entitlement.rs`, `services/sharing_authz.rs`, `services/invitation_service.rs`, `services/reconcile.rs`, `services/sharing.rs`, `mappers/calendar.rs`, `mappers/calendar_invitation.rs`, `mappers/calendar_editor.rs`, `mappers/subscription.rs`, `mappers/user.rs`, `mappers/password_reset.rs`, `redis_pubsub.rs`, `cache.rs`, `metrics/exporter.rs`, all migrations.

**Frontend:** `App.vue`, `router/index.ts`, `config/api.ts`, `main.ts`, `stores/auth.ts`, `components/PrivacyPolicyPage.vue`, `TermsOfServicePage.vue`, `ui/AppFooter.vue`, `InviteLandingPage.vue`, `LoginPage.vue`, `MembersTab.vue`, `CalendarEditorViewDesktop.vue`, `UpgradePage.vue`, `UpgradeInterruptModalBody.vue`, `composables/useUpgradeInterrupt.ts`, `usePresence.ts`, `index.html`, dirty-file diffs for App.vue / locales / router. Targeted greps across all of `frontend/src` for `v-html`, `innerHTML`, `console.*`, `target=_blank`, `as any`, `@ts-ignore`, `window.location`, DaisyUI legacy classes, `vi.mock('vue-router')`, `createWebHistory`, `data:`/`javascript:`, `MutationObserver`, lodash/moment/date-fns, `errorCaptured`, `redirect=`, `<img>` src patterns.

**Infra:** `Dockerfile`, `frontend/Dockerfile`, `docker-compose.yml`, `ops/docker-compose.monitoring.yml`, `frontend/nginx.conf`, `.github/workflows/ci.yml`, all `*.toml` config files (real + `.dist`), `redocly.yaml`, `Cargo.toml` workspace + `server/Cargo.toml`, `frontend/package.json`, `Cargo.lock` (selected entries).

### Files / surfaces NOT covered (suggested follow-up)
- `anilist/` and `common/` workspace crates — out of scope by initial direction; worth a separate spot-check.
- Pinia stores other than `auth.ts` — `userSettingsStore`, `usageStore`, `editorSelection`, `sharingStore` — for dead-field sweep.
- `CalendarEditorViewMobile.vue` past the actor-filter section (mirror of Desktop with same H-1/H-2 surface).
- `frontend/e2e/*.spec.ts` — E2E semantics, not security; would catch Phase 3 regressions.
- `services/{presence,email_validation,calendar_events,frozen_ics}.rs`, `mappers/{anilist,database,google_oauth,user_settings,refresh_token,email_verification,stripe_event}.rs` — covered indirectly through call-sites; deeper read recommended for the next audit pass.
- Docker image runtime characteristics (image size, attack-surface scan via Trivy/Grype) — static-analysis-only first pass per agreement.
- Anilist HTML synopsis rendering path — not yet implemented as user-visible HTML rendering, hence no `v-html` finding; flagged for re-audit if/when synopsis is surfaced.
- Live/runtime verification of any flow — explicitly out of scope per agreement; recommend Playwright sweep on auth + sharing as a follow-up after Phase 1 fixes.

---

## Cumulative-review additions (Phase 1, 2026-05-07)

Surfaced by the end-of-Phase-1 cumulative reviewer; not part of the original parallel-subagent sweep. File these as Phase 2+ candidates.

- **CR-1 (MEDIUM)** — `services/ics_export.rs`: `.summary()` (lines ~265, ~268) and the VALARM `.description()` pass AniList-sourced anime titles to the `icalendar` builder. The crate handles RFC 5545 escaping (commas, semicolons, backslashes) but does **not** strip CR/LF. Server-controlled today (titles come from AniList, not user input), but if AniList ever returns a title containing `\r\n`, both `SUMMARY` and the `VALARM DESCRIPTION` are injectable. **Fix:** wrap with `sanitize_name_for_line_protocol` defensively. Same one-liner pattern as the H-1 follow-up at line 279.
- **CR-2 (LOW)** — `frontend/src/__tests__/App.spec.ts`: the stub router built by `makeRouter()` does not include `/privacy` or `/terms`, so `<RouterLink>` in `AppFooter` emits Vue Router warnings on every test run (tests still pass; warnings are cosmetic). Pre-existing from the `feat(legal)` commit on main; not introduced by Phase 1.
- **CR-3 (LOW)** — `frontend/src/components/account/DangerZoneTab.vue`: no unit tests for the logout flow. After the T6 refactor (`logout(push?)`), there is now an injection seam that would make a unit test trivial — but no test exists. Latent risk for future regressions.

## Phase 2 deferred advisories — cargo-deny (2026-05-07)

Added `cargo-deny` CI gate (M-22). First-run `cargo deny check` found **13 current advisories**, all deferred to Phase 3 via `[advisories.ignore]` in `deny.toml`. CI is green from day one; any new advisory going forward fails CI.

- **CD-1 (HIGH)** — `RUSTSEC-2026-0044`: AWS-LC X.509 Name Constraints Bypass via Wildcard/Unicode CN. Affected dep: `aws-lc-sys 0.35.0` (via `jsonwebtoken`, `rustls`). **Fix:** `cargo update -p aws-lc-sys` to >=0.39.0.
- **CD-2 (HIGH)** — `RUSTSEC-2026-0045`: Timing Side-Channel in AES-CCM Tag Verification in AWS-LC. Affected dep: `aws-lc-sys 0.35.0`. **Fix:** upgrade to >=0.38.0.
- **CD-3 (HIGH)** — `RUSTSEC-2026-0046`: PKCS7_verify Certificate Chain Validation Bypass in AWS-LC. Affected dep: `aws-lc-sys 0.35.0`. **Fix:** `cargo update -p aws-lc-sys`.
- **CD-4 (HIGH)** — `RUSTSEC-2026-0047`: PKCS7_verify Signature Validation Bypass in AWS-LC. Affected dep: `aws-lc-sys 0.35.0`. **Fix:** `cargo update -p aws-lc-sys`.
- **CD-5 (HIGH)** — `RUSTSEC-2026-0048`: CRL Distribution Point Scope Check Logic Error in AWS-LC. Affected dep: `aws-lc-sys 0.35.0`. **Fix:** `cargo update -p aws-lc-sys`.
- **CD-6 (MEDIUM)** — `RUSTSEC-2026-0009`: Integer overflow in `BytesMut::reserve`. Affected dep: `bytes` (transitive). **Fix:** `cargo update -p bytes` to >=1.10.1.
- **CD-7 (MEDIUM)** — `RUSTSEC-2026-0007`: Denial of Service via Stack Exhaustion. Affected dep: transitive. **Fix:** `cargo update` to pick up patched version once available.
- **CD-8 (MEDIUM)** — `RUSTSEC-2026-0097`: Rand is unsound with a custom logger using `rand::rng()`. Affected dep: `rand 0.9.2` (transitive via `actix-http`). **Fix:** upgrade rand to >=0.9.3 or >=0.10.1.
- **CD-9 (HIGH)** — `RUSTSEC-2023-0071`: Marvin Attack — potential key recovery through timing sidechannels. Affected dep: `rsa 0.9.8` (via `google-oauth`). No safe upgrade available yet. Low exploitability for our use case (Google ID token verification only). **Fix:** track upstream `RustCrypto/RSA` for patch; consider switching to `google-oauth` version that uses `ring` or `aws-lc-rs` instead of `rsa`.
- **CD-10 (HIGH)** — `RUSTSEC-2026-0049`: CRLs not considered authoritative by Distribution Point due to faulty matching logic. Affected dep: `rustls-webpki 0.103.4` (via `rustls`). **Fix:** `cargo update -p rustls-webpki` to >=0.103.10.
- **CD-11 (HIGH)** — `RUSTSEC-2026-0098`: Name constraints for URI names were incorrectly accepted. Affected dep: `rustls-webpki 0.103.4`. **Fix:** `cargo update -p rustls-webpki`.
- **CD-12 (HIGH)** — `RUSTSEC-2026-0099`: Name constraints accepted for certs asserting wildcard name. Affected dep: `rustls-webpki 0.103.4`. **Fix:** `cargo update -p rustls-webpki`.
- **CD-13 (HIGH)** — `RUSTSEC-2026-0104`: Reachable panic in CRL parsing. Affected dep: `rustls-webpki 0.103.4`. **Fix:** `cargo update -p rustls-webpki`.

**CD-LICENSE-1 (HIGH — blocks commercial distribution)** — `actix-governor 0.7.0` is GPL-3.0-or-later (verified in upstream Cargo.toml; the underlying `governor` crate is MIT). Linking GPL-3.0 code into a closed-source binary is incompatible with proprietary distribution. SaaS-only network use is technically permitted by GPL-3.0 (this is what AGPL adds), but most legal teams refuse GPL-3.0 in product code regardless. **Phase 3 must treat this as HIGH priority and replace the dep before any commercial closed-source release.** Replacement options: (a) use `governor` directly with a thin in-house actix wrapper, (b) switch to a permissively-licensed actix rate-limiter (e.g. `actix-extensible-rate-limit` if MIT/Apache-2.0 — verify before adopting), (c) accept GPL-3.0 if the user is comfortable distributing the project under GPL-3.0 terms.

**RESOLVED 2026-05-07** — replaced with in-house MIT-licensed wrapper around `governor` 0.10 (see `server/src/middleware/rate_limit.rs`). `actix-governor` removed from `server/Cargo.toml`; `[[licenses.exceptions]]` entry removed from `deny.toml`. IPv6 /56-prefix bucketing and webhook-exempt path semantics are fully preserved. `cargo deny check` exits 0 with no GPL crates in the tree.

---

## Phase 1 status (2026-05-07 — fully shipped)

8 audit findings closed across 14 commits on `audit/phase-1-cheap-and-critical` (merged to main):
- **CRITICAL**: C-2 SSE actor filter (T1 backend + T2 frontend, 5 commits).
- **HIGH**: H-1 + H-2 ICS / Content-Disposition CRLF (T3 + 1 cumulative-fixup commit). H-5 invitation 403 → 400 anti-enumeration (T4, 2 commits). H-9 protocol-relative redirect (T5). H-11 forbidden test patterns + `vi.mock('vue-router')` removal via DI (T6, 2 commits).
- **MEDIUM**: M-25 cookie_secure prod startup assert (T7).
- **LOW × 2**: L-2 + L-3 noisy INFO logs demoted to DEBUG (T8).

C-1 (account-delete abandons Stripe sub) and the rest of the HIGH list (H-3, H-4, H-6, H-7, H-8, H-10, H-12 through H-18) remain on the Phase 3+ docket per the `Recommended fix sequence` above.

## Phase 2 status (2026-05-07 — fully shipped)

7 audit findings closed across 8 commits on `audit/phase-2-config-only` (merged to main):
- **HIGH**: H-12 nginx security headers (CSP/HSTS/XFO/XCTO/Referrer/Permissions). H-13 backend CSP scope verification. H-14 SSE-friendly `/api/` proxy config (`proxy_buffering off` + `proxy_read_timeout 1h` + `Connection ""` + `chunked_transfer_encoding off`). H-15 docker-compose Postgres + Redis bound to 127.0.0.1.
- **MEDIUM**: M-18 `config.docker.toml.dist` synced with `config.toml.dist`. M-19 Rust base bumped 1.85 → 1.95.0; debian runtime digest-pinned. M-20 Dockerfile gains `HEALTHCHECK` (probes `/public-config`) + `tini` PID-1 reaper. M-21 `@redocly/cli` pinned to `2.30.4` (no more `@latest`). M-22 `cargo-deny` CI gate with `deny.toml` (license + advisory + bans + sources).

CSP profile shipped is **pragmatic** (`'unsafe-inline'` for script-src to keep the pre-paint inline theme bootstrap working without a build-time hash plugin); strict-CSP-with-hashes is on the Phase 3 docket.

## Phase 2 cumulative-review additions

Surfaced by the end-of-Phase-2 cumulative reviewer; file as Phase 3 candidates.

- **CR-P2-1 (LOW)** — `database.docker.toml.dist` does not exist alongside `config.docker.toml.dist`. `docker-compose.yml` mounts both `./config.docker.toml` and `./database.docker.toml`, so a developer cloning the repo and following the docker-compose path needs a template they don't have. Fix: add the dist file with the same shape the project's `database.toml.dist` uses, adjusting `host = "postgres"` and the credentials for the docker-compose service.
- **CR-P2-2 (LOW)** — `CLAUDE.md` has no documentation for `cargo deny`. The "Build & Run" / "CI/CD" sections should describe how to run `cargo deny check` locally and how to add a new advisory ignore (RUSTSEC ID + Phase 3 reason comment) when an advisory genuinely cannot be addressed immediately.
- **CR-P2-3 (NIT)** — `deny.toml` has two `license-not-encountered` warnings (`MPL-2.0`, `Unicode-DFS-2016` are in `[licenses.allow]` but no crate currently uses them). Either remove the unused entries to keep the allowlist tight, or add a comment naming a future dep that is expected to use them.

## Phase 2 deferred advisories — cargo-deny

13 RUSTSEC advisories surfaced by the first cargo-deny run are deferred via `[advisories.ignore]` in `deny.toml`. Each is a Phase 3 candidate (5 × aws-lc-sys, 4 × rustls-webpki, 4 × misc, 1 × rsa Marvin attack via google-oauth). See the `# Phase 3:` comments in `deny.toml` for the per-advisory remediation path. **CD-LICENSE-1 (HIGH — blocks commercial closed-source distribution)**: ~~`actix-governor 0.7.0` is GPL-3.0-or-later — the only direct copyleft dependency. Phase 3 must replace it before any commercial closed-source release. Replacement options documented in `deny.toml` exception comment.~~ **RESOLVED 2026-05-07** — see CD-LICENSE-1 resolution note above.

## Phase 3 status (2026-05-07 — fully shipped)

6 audit findings closed across 11 commits on `audit/phase-3-critical`:

- **HIGH (license)**: CD-LICENSE-1 — `actix-governor 0.7.0` (GPL-3.0-or-later) replaced with an in-house MIT-licensed actix middleware wrapping the MIT `governor` crate. Same 60-burst / 1-rps semantics, `/stripe/webhook` exempted, byte-identical 429 body. `deny.toml` exception removed (T1, 2 commits).
- **CRITICAL**: C-1 — `delete_user` now opens a transaction, checks for an active subscription via `SubscriptionMapper::find_active_for_user_with`, and returns `409 Conflict` `{"error":"active_subscription"}` if found. Frontend `DangerZoneTab` catches the 409 and surfaces a Stripe Portal modal. New `account.danger.activeSubscription.*` i18n keys (en + pt). 5 frontend tests + 3 backend integration tests (T2, 3 commits + 1 typecheck fixup).
- **HIGH**: H-7 + H-8 — Reconcile `apply_reconcile_update` WHERE clause tightened to also reject equal-period status regressions; `dispatch_event` gains handlers for `customer.subscription.paused`, `customer.subscription.resumed`, `customer.deleted`, and `invoice.payment_action_required`. New mapper helper `update_all_status_by_customer_in_tx`. New metric `STRIPE_WEBHOOK_PAYMENT_ACTION_REQUIRED_TOTAL` (T3, 2 commits).
- **HIGH**: H-4 — `safe_url` access-log replacer extracted to `redact_path` free fn and extended to scrub `/invitations/{token}[/action]`. nginx adds `^~ /invite/` and `^~ /api/invitations/` location blocks with `access_log off;` so neither the SPA landing page nor the proxied API calls leak the token to nginx stdout. 5 unit tests on `redact_path` (T4, 1 commit).
- **HIGH**: H-6 — New `SseConnectionTracker` service (`Mutex<HashMap<i32, usize>>` behind `Arc`) caps concurrent SSE connections per user. Default 8, configurable via `SharingConfig.sse_max_connections_per_user`. Handler returns `429 Too Many Requests` once at cap. RAII guard moved into the `async_stream::stream!` block so it lives for the connection lifetime. 5 tracker unit tests + handler test updated for new extractor (T5, 1 commit).
- **HIGH**: H-10 — Invite token no longer rides along in `?redirect=/invite/{token}` query (which leaked via history, bookmarks, browser sync, and tab-read extensions). Replaced with `sessionStorage` stash via new `composables/inviteRedirect.ts`. `InviteLandingPage`'s 3 sign-in / sign-up router-links became `<button>`s that stash + push plain `/login` or `/register`. `LoginPage` consumes the stash on success and clears it. New helper unit tests + 4 component-spec assertions (T6, 1 commit + the typecheck fixup folded with T2).

Final verification (post-T6):
- `cargo test -p server --lib` — 358/358 green, idempotent.
- `cargo check --workspace` — clean.
- `npm --prefix frontend run test:unit` — 479/479 green.
- `npm --prefix frontend run build` — clean (vue-tsc + rolldown).
- `npm --prefix frontend run lint` — clean.

The remaining HIGH list (H-3 advisory-lock bypass on `add_item`; H-12 through H-18 from the "Phase 4 — Quality & DRY" track) and the cumulative-review additions from Phase 2 (CR-P2-1, CR-P2-2, CR-P2-3) remain on the docket.

---

*Generated by 6 parallel specialist subagents on 2026-05-07. Each agent's full transcript is preserved in the task system; this document is the synthesis. Cumulative-review additions and Phase status sections appended after each phase ships.*

---

# Follow-up audit — 2026-06-11

**Scope:** (a) verification of the remaining 2026-05-07 docket; (b) the surfaces the original audit explicitly skipped (`anilist`/`common` crates, `services/{presence,email_validation,calendar_events,frozen_ics,cached_anilist}.rs`, mappers `anilist/database/google_oauth/user_settings/refresh_token/email_verification/stripe_event`, `cache.rs`/`redis_pubsub.rs`, metrics, bins, `rate_limit.rs`, non-auth Pinia stores, `CalendarEditorViewMobile`, composables, services layer, e2e specs); (c) fresh supply-chain, semgrep, DB-index, and N+1 sweeps.
**Method:** 4 parallel read-only subagents (docket verifier, backend-uncovered, frontend-uncovered, deps/perf/infra). No code changed during the audit.

## Docket verification (2026-05-07 items)

**Fixed since:** M-14 (public-route prefixes), CR-P2-1 (`database.docker.toml.dist`), L-1 (refresh-token invalidation in `delete_user`). **Partially fixed:** M-26 (webhook dispatch-primitive tests exist at `stripe_webhook.rs:708+`; still no signed-payload `Webhook::construct_event` fixture test; `services/email.rs`, `controllers/sharing.rs`, `controllers/stripe.rs` still untested).

**Confirmed still open** (current line refs): H-3 (`controllers/calendar.rs:1097-1102` — `add_item` cap check pool-bound, no advisory lock; contrast `put` at `:619/:638`), H-16 (`item.rs:6-19`), H-17 (37 `_with` fns across 7 mapper files), H-18 (`email.rs:28,83,147,211`), M-1 (`middleware/auth.rs:47`, `services/auth.rs:121` — `Validation::default()`, no alg pin / required claims), M-2/M-3 (`error.rs:39-44` + catch-all `:97-99` serializes inner strings to clients), M-4 (`oauth.rs:59`), M-6 (`sse.rs:108,112`), M-9 (`reconcile.rs:8-11` lock still "Deferred"), M-10 (`en.json:409,480-481` + pt), M-11 (`stripe.rs:112` set, never read), M-15 (`stores/auth.ts:11-12,94-98`), M-23 (19 up-only migrations), M-24 (now F2-1, upgraded to confirmed bug), M-29 (`sharing.rs:49,122`), M-30 (4 copies incl. `controllers/sharing.rs:71-92`), CR-1 (`ics_export.rs:244-268` — SUMMARY/VALARM still unsanitized; only calendar name wrapped), L-5, L-8, L-11.

## New findings

### HIGH

- **F2-1 — Rate limiter keys on TCP peer IP; one global bucket behind nginx.** `middleware/rate_limit.rs:141` uses `req.peer_addr()`, never `X-Forwarded-For`/`realip_remote_addr()`. In the shipped nginx topology all clients share one 60-burst/1-rps bucket: no per-abuser throttling, and a drained burst 429s everyone. IPv6 /56 bucketing itself is correct; it buckets the wrong address. Closes M-24's "verify". Fix: trusted-proxy config flag → key on rightmost trusted `X-Forwarded-For` hop, fall back to peer IP. Nit: missing-peer branch returns non-convention 500 (`:150-158`).
- **F2-2 — NEW RUSTSEC-2026-0141 (lettre 0.11.21) fails cargo-deny; CI red on next push.** Boring-TLS-only hostname-verification bug — not exploitable here (`tokio1-native-tls`), but not ignored either. Fix: `cargo update -p lettre` (>=0.11.22).
- **F2-3 — AniList batch query silently truncates at 25 items.** `anilist/queries/get_items.graphql:2` — `Page { media(id_in: $ids) }` with no `perPage` (AniList default 25, max 50). Pro users with >25 unique shows across visible calendars silently lose items from `/calendars` and ICS export. Fix: `perPage: 50` + chunk ids in `client.rs::get_items`. Related: `page_size` unclamped (`controllers/calendar.rs:344-354`).
- **F2-4 — EventSource has no `onerror`: live sync (incl. kick frames) dies permanently and silently.** `usePresence.ts:39-44` — 401 after cookie expiry or 429 from the H-6 cap fails the connection with no browser auto-reconnect, no re-auth, no UI hint. Fix: `onerror` → close, authed ping via `api` (triggers refresh interceptor), reconnect with backoff.
- **F2-5 — E2E spec asserts the removed H-10 token-leak behavior.** `e2e/sharing-invite-landing.spec.ts:37-43` expects `href="/login?redirect=/invite/{token}"` on elements that became `<button>`s — suite broken post-H-10. Companion: e2e `/api/user` stubs return `{id,...}` but `auth.ts` reads `user_id`, so `authStore.userId` is null in every e2e run.
- **F2-6 — AniList client ignores HTTP status; no 429/`Retry-After`/backoff.** `anilist/src/client.rs:45-54,83-92,121-130,163-172` — `res.json()` without `error_for_status()`; throttled responses surface as opaque deserialize errors while the server keeps hammering upstream.

### MEDIUM

- **F2-7 — SSE `Err(_) => {}` swallows `RecvError::Closed` → per-connection 100%-CPU hot loop** if a fan-out sender drops (`sse.rs:108,112`; kick arm `:123`). Fix: `break` on `Closed`; emit `{"type":"resync"}` on `Lagged` (folds in M-6).
- **F2-8 — Google OAuth never checks `email_verified`** (`mappers/google_oauth.rs:27-39`, `controllers/oauth.rs:59-73`) — unverified Google email is trusted, marked verified, and linked to any matching local-password account (concretizes M-4). Fix: reject `email_verified != true`; confirmation step before first link.
- **F2-9 — Cross-user settings-cache leak on login.** `services/userSettingsService.ts:21-38` + `stores/auth.ts:46-106` — 30-min `user_settings` localStorage cache only invalidated on explicit `logout()`; cookie-expiry → new login serves the previous user's settings incl. `user_id` (feeds `isOwner`). Fix: invalidate on every successful login path.
- **F2-10 — Router guard re-clobbers theme/locale with fabricated defaults** (`router/index.ts:144-149` + `userSettingsStore.ts:26-27,43-45`) — the documented `feedback_unauth_default_reconcile` hazard, fixed in `main.ts` but alive here; transient settings 500 does the same for authed users. Related: `applySettings.ts:4-12` mutates `data-theme` behind `useTheme`'s back → toggle desync.
- **F2-11 — Mobile editor draft persistence is a TODO but unmount still deletes the desktop draft.** `CalendarEditorViewMobile.vue:196-207` — dead timer var, no watcher, yet `onBeforeUnmount` removes `calendarPageState`; viewport flips lose edits. Desktop restore also not keyed by calendar id (`Desktop.vue:265-278` — failed fetch shows calendar A's draft under `/calendar/B`). Root cause: ~140 duplicated lines between variants; extract a shared composable.
- **F2-12 — SSE toasts render raw numeric user ids** ("42 added an item") — both editor variants interpolate `frame.actor` (now `actor_id` post-C-2) into `sharing.toasts.*`. Fix: resolve id → display name, or add a `display` field to item frames.
- **F2-13 — Presence heartbeat bypasses the `api` instance** (`usePresence.ts:42` uses global `axios`) — no 401-refresh; user silently drops from viewers list after token expiry.
- **F2-14 — Missing DB indexes on hot paths:** `subscriptions.stripe_customer_id` (webhook SELECT/UPDATE, `mappers/subscription.rs:261,309`) and `calendar_invitations (inviter_id, sent_at)` (invite rate-limit COUNTs, `calendar_invitation.rs:334-338,379-385`).
- **F2-15 — DB/Redis passwords interpolated raw into connection URLs** (`mappers/database.rs:16-24`, `cache.rs:37-41`, `presence.rs:35-39`, `redis_pubsub.rs:28-35`, `bin/set_subscription.rs:306-313`) — `@:/#` in a password mis-parses. Fix: structured connect options / percent-encode.
- **F2-16 — `Cache::new` ignores its `db` param; `flush()` is `FLUSHALL`** (`cache.rs:36-47,135-146`) — pub footgun that would wipe the whole Redis server. Fix: honor `db` + `FLUSHDB`, or remove both.
- **F2-17 — Search cache key delimiter collision** (`cache.rs:307-312`) — `search:{query}:{type}` from raw query; `name="foo:ANIME"` collides. Fix: hash the query component.
- **F2-18 — `Error::Redis(String)` leaks internals to clients** (`error.rs:69-70` via `:97-99`) — same class as M-2/M-3.
- **F2-19 — axios 1.13.2 (prod) carries the prototype-pollution/CRLF advisory family** → bump 1.17.0 (in-range; re-run auth interceptor specs). vitest 4.0.15 has a critical (UI server file read/exec) → 4.1.8. `npm audit`: 3 prod / 13 total vulns.
- **F2-20 — AniList client logs full upstream payloads at INFO** (`anilist/src/client.rs:66,77,102,115,141,156,184`) — demote to debug/trace.

### LOW

- **F2-21 — `deny.toml`/AUDIT.md swap RUSTSEC-2026-0007↔0009**: 0007 = bytes (fix >=1.11.1), 0009 = **time** (fix >=0.3.47). CD-6's ">=1.10.1" is stale.
- **F2-22 — Redis N+1 in `cached_anilist.rs:84-102`** — 2 serial GETs per item per `/calendars` request; use MGET/pipeline.
- **F2-23 — Loop-invariant tier check re-queried per item** in PUT pre-check loop (`controllers/calendar.rs:591-595,636-643` via `entitlement.rs:92-94,164-166`); hoist.
- **F2-24 — `users.username` still a full UNIQUE** while email/subscription_token use soft-delete partial uniques — deleted accounts block username reuse forever; decide intent.
- **F2-25 — Redundant indexes**: `idx_calendar_items_calendar_id` (prefix of PK) and `idx_calendars_subscription_token` (shadowed by partial) — drop both.
- **F2-26 — `set_subscription.rs:210-247` DELETE+INSERT without a transaction** (dev tool; convention violation).
- **F2-27 — `Language::from_str` maps unknown values to `Native`** (`common/src/language.rs:15-25`) instead of the declared `#[default] English` or an error.
- **F2-28 — Frontend store hygiene cluster**: dead `?session_id=` param still sent (`services/subscription.ts:34-39`); `UpgradeSuccessPage.vue:44` 800ms redirect timer untracked past unmount; `useCalendarSearch.ts:14-32` + `sharingStore.loadMembers` lack stale-response guards; `usageStore.reset()` dead + caps hardcoded (`usageStore.ts:10-11`); `userSettingsStore.error` write-only hardcoded-English; `editorSelection` doc claims desktop uses it (it doesn't); `useTheme.init()` unremovable storage listener.
- **F2-29 — `/metrics` exporter network-gated only** — document/assert loopback-or-private binding (label cardinality verified bounded).
- **F2-30 — CI nits**: `openapi` job `setup-node` missing `cache: npm`; clippy allow-list lives on the CI command line instead of the canonical `[lints.clippy]` table.

### Verified clean (don't re-audit)

GraphQL injection (typed variables only); `common/` panic-free with guarded casts; `presence.rs` SCAN+TTL bounded; pub/sub reconnect backoff + `Weak` teardown; rate-limit key-map eviction; metrics label cardinality; semgrep 0 findings (305 files — weak signal for Rust); CI actions all SHA-pinned, no pipe-masked tests, caching present; i18n en/pt key sets exactly symmetric; e2e catch-all route ordering correct; AniList lookups batched one-call-per-page; listener hygiene in Ui primitives.

## Recommended sequence

1. **Same-day batch:** F2-2 + deferred-advisory `cargo update` batch (`lettre rustls-webpki bytes time rand@0.9.2 aws-lc-sys`) + prune `deny.toml` to the rsa Marvin entry + fix F2-21 labels; F2-19 npm bumps + `npm audit fix`; F2-5 e2e spec + stub shape; F2-14 index migrations; F2-7 `Closed` → `break`.
2. **Small high-value PRs:** F2-1 trusted-proxy keying; F2-8 `email_verified`; F2-4 + F2-13 SSE/heartbeat recovery; F2-9 cache invalidation on login; F2-3 + F2-6 AniList `perPage`/chunking + status handling.
3. **Refactor:** shared editor composable (F2-11/F2-12 surface), then docket H-3 and F2-10.
4. **Backlog:** remaining docket (H-16/17/18, M-*, CR-1, L-*) + F2 LOWs.

## Same-day batch status (2026-06-11 — fully shipped)

Sequence step 1 closed across 5 commits on `main`:

- **F2-2 + deferred advisories**: `cargo update` batch (aws-lc-sys 0.41.0, rustls-webpki 0.103.13, bytes 1.11.1, time 0.3.47, rand 0.9.4, lettre 0.11.22); `deny.toml` ignore list pruned to RUSTSEC-2023-0071 only; F2-21 label swap corrected. `cargo deny check advisories` ok.
- **F2-7**: SSE stream breaks on `RecvError::Closed` for all three channels (was a per-connection CPU hot loop if a fan-out sender dropped).
- **F2-14**: `idx_subscriptions_stripe_customer_id` + `idx_calendar_invitations_inviter_sent` migration; applied to dev DB manually with `_sqlx_migrations` bookkeeping (checksum drift still blocks `sqlx migrate run`).
- **F2-19**: axios 1.17.0 + vitest 4.1.8 + `npm audit fix` → 0 npm vulnerabilities.
- **F2-5 (expanded)**: the e2e suite was failing **6/39** pre-existing (never caught — Phase 3 verification only ran unit tests). Three distinct spec bugs fixed: stale pre-H-10 href assertions; the mismatch test's stub glob never matched the real `/accept` POST path (fell through to the catch-all abort); downgrade-kick waited on a desktop-only marker on mobile-viewport projects *and* raced the instant stubbed kick redirect. Plus `MOCK_USER` stub shape (`id` → `user_id`) in both specs.

Verification (sequential): `cargo deny check advisories` ok · `cargo test -p server --lib` 358/358 · clippy no new warnings · `npm run test:unit` 479/479 · lint + build clean · `npm run test:e2e` **39/39** (was 33/39).

Remaining from the follow-up sequence: step 3 (shared editor composable, docket H-3, F2-10), step 4 backlog.

## Step 2 status (2026-06-11 — fully shipped)

7 findings closed across 5 commits on `main`:

- **F2-8** (`523114a`): Google OAuth rejects tokens whose email is not verified (`email_verified != Some(true)` or missing email → `Error::Unauthorised`); payload conversion extracted to a testable helper, 4 unit tests.
- **F2-1** (`d63f90e`): rate limiter keys on the rightmost `X-Forwarded-For` entry when the new `trust_proxy_header` config flag is set (default false; `config.docker.toml.dist` sets true — **the live production config must add it too**). Peer-IP fallback retained; missing-peer 500 body genericised; 3 new tests.
- **F2-3 + F2-6 + F2-20** (`a0b7475`): AniList `get_items` pins `perPage: 50` and chunks id batches (>25 ids no longer silently truncated); new generic `execute<Q>()` handles HTTP status — 429 with short `Retry-After` retries once, else typed `RateLimited`/`HttpStatus` errors; payload logs demoted to debug. Server clamps `GET /calendars` `page`/`page_size` (page=0 previously underflowed the OFFSET computation).
- **F2-9** (`8763669`): all three login paths invalidate the localStorage settings cache, closing the cross-user leak after cookie-expiry session ends.
- **F2-4 + F2-13** (`08e4f6c`): EventSource `onerror` → backoff reconnect (1s→30s) with authed `GET /user` ping to drive the 401-refresh interceptor; dead session stops cleanly. Heartbeat moved from bare axios to the `api` instance. `MockEventSource` extended; 5 new tests.

Verification (sequential): `cargo test --workspace --lib` 365/365 · clippy clean in touched files · `npm run test:unit` 484/484 · lint + build clean · `npm run test:e2e` 39/39.

## Step 3 status (2026-06-11 — fully shipped)

4 findings closed across 5 commits on `main` (2 substantive commits + 3 review fixups). Design + plan at `docs/superpowers/specs/2026-06-11-step-3-editor-composable-design.md` and `docs/superpowers/plans/2026-06-11-step-3-editor-composable.md`.

- **F2-12 + H-3** (backend — `f142721`, fixup `25b8c64`): `CalendarEvent::ItemAdded`/`ItemRemoved` SSE frames gain a `display` field (the actor's username, sourced via `UserMapper` like `MemberJoined`); item toasts no longer render raw numeric ids. `add_item` now runs its owner show-cap check + insert inside a `pg_advisory_xact_lock(owner)` transaction (the `put` pattern), closing the concurrent-add cap bypass. New `owner_at_show_cap_cannot_add_new_item_returns_402` regression test (asserts 402 on new-at-cap + 200 on already-tracked); item test harness wires the new `pool` + `UserMapper` Data; serialization test asserts `display` is emitted. `controller_pool` comment in `main.rs` updated to note it now backs both advisory-locked handlers.
- **F2-11 + F2-12 + F2-10** (frontend — `4bf6f6e`, fixups `a65c01f` test restore, `1b35b91` draft-race + dead-export): new `composables/useCalendarEditor.ts` orchestrator owns the editor core shared verbatim between desktop + mobile (state, submit, SSE watcher, presence, bootstrap, draft persistence); both views are now thin search-glue + template. Drafts are keyed `calendarDraft:{id}` (no cross-calendar bleed; portable across viewport flips; overlaid after server load so unsaved edits win), persisted on both views, cleared on submit. Item toasts use `frame.display`. Router guard mirrors `main.ts`: auth-guarded `useTheme().reconcileFromServer()` + locale instead of the unauth-clobbering `applySettings` path; `applySettings` retained for its 3 other callers.

Verification (sequential): `cargo test -p server --lib` 366/366 · clippy clean in touched files · `npm run test:unit` 485/485 (clean run) · lint + build clean · `npm run test:e2e` 39/39 (incl. `viewport-flip.spec.ts`, which exercises the shared composable + keyed drafts in the real app).

Two items deferred to the backlog (Step 4):
- **F2-31 (LOW, perf)** — the `display` username lookup in `add_item`/`remove_item` runs a DB round-trip on every successful mutation, even when the calendar has no SSE subscribers (`controllers/calendar.rs`). Surfaced by the Step-3 code review (I-2). Fix would resolve `display` lazily inside the publisher (gate on `has_subscribers`) or pass `actor_id` through and let the publish path resolve it. Deferred deliberately because it changes the `CalendarEventPublisher` API; the backend-`display` approach was the chosen design.
- **F2-32 (LOW, test infra)** — the full `npm run test:unit` run is order-flaky: `ResetPasswordPage.spec.ts` (and previously `SubscriptionTab`) fail intermittently with `vi.mocked(...).mockResolvedValue is not a function`, a cross-file `vi.mock('axios')` registry leak (the documented `feedback_vue_router_mock_leaks_across_workers` class). Both pass in isolation; a clean full run is 485/485. Pre-existing; not introduced by Step 3 (which never mocks `axios`). Root-cause fix is per-file mock isolation / `vi.resetModules` discipline — its own task.

Remaining from the follow-up sequence: step 4 backlog (H-16/17/18, M-*, CR-1, L-*, F2 LOWs incl. the two above).

## Step 4 — security + cheap HIGH wins batch (2026-06-11 — fully shipped)

6 findings closed across 4 commits on `main`. Implemented directly (small prescribed fixes), TDD where testable, then an adversarial security review (Approved). Backend suite 370/370 · zero new clippy warnings in touched files.

- **M-2 + M-3 + F2-18** (`a555b2c`): `Error::error_response()` now redacts the three variants whose `Display` interpolates an internal detail string — `Stripe`, `EmailError`, `Redis`. Each logs the full detail server-side (`log::error!`) and returns a stable generic code (`stripe_error` / `email_error` / `internal_error`) instead of leaking SMTP / Stripe / Redis internals to clients. Full enum audit (in the review) confirmed no other interpolated variant leaks via the `self.to_string()` fallback; the `#[from]` variants all have static Display strings. 3 leak-assertion tests.
- **M-1** (`f54d2b7`): JWT validation in both the `Claims` `FromRequest` extractor and `services::auth::verify_token` pins the algorithm explicitly (`Validation::new(Algorithm::HS256)`, no alg-confusion) and requires all issued registered claims (`set_required_spec_claims(&["exp","sub","iss","aud"])`). `generate_token` is the sole issuer and emits all four, so no valid token regresses.
- **CR-1** (`ba79c75`): the per-episode `summary` (AniList item title) is wrapped in `sanitize_name_for_line_protocol` before flowing into both `event.summary()` and the VALARM `Alarm::display()` DESCRIPTION — closing the last CR/LF injection vector in `render_common_calendar` (the calendar name was already sanitized in the H-1 follow-up). New injection-guard test on the item title.
- **H-16** (`58f00ea`): explicit `operation_id = "get_item"` on the bare `item.rs::get` handler; review confirmed no collision with existing operation ids.

New backlog item surfaced by the review:
- **F2-33 (LOW, semantics)** — `Error::CannotHashPassword` and `Error::CannotGenerateAuthToken` map to `400 Bad Request` in `error.rs::status_code`, but they are server-side crypto failures and should be `500`. No leak (static Display); pre-existing. Fix would move both arms to the `INTERNAL_SERVER_ERROR` group (verify no register/login test asserts 400 on these paths first).

Remaining step 4 backlog: H-17 (`_with`→`_in_tx` mapper rename), H-18 (email send-fn extraction), the M-29/M-30 quality items, F2 perf/DB cleanup (F2-22/23/24/25), F2-28 frontend store hygiene, F2-31/F2-32/F2-33, and the L-* nits.
