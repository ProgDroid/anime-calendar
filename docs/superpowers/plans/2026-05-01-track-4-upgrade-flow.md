# Track 4 — Upgrade Flow implementation plan

**Date filed:** 2026-05-01
**Status:** Pending — start with Phase 1
**Spec:** `docs/superpowers/specs/2026-05-01-track-4-upgrade-flow-design.md`
**Predecessor tracks:** 1 ✅, 2 ✅
**Estimated total effort:** ~2 weeks of focused work, 6 phases each independently shippable.

This plan turns the Track 4 spec into ordered, ship-able batches. Phases are sequential — each builds on the previous one's contract. Within a phase, tasks can be done in any order. Each phase produces a working app at the end (no half-built feature flags).

## Pre-flight (before Phase 1)

- [ ] Confirm provisional pricing values for the dev/staging Stripe Dashboard (e.g. $4.99/mo, $49.90/yr → 2 months free). Production pricing can be filled later, this is only for the test-mode price IDs the dev environment will use.
- [ ] Create Stripe test-mode account (or reuse existing) and capture: `pk_test_...`, `sk_test_...`, `whsec_...`, `price_..._monthly`, `price_..._annual`. Store in `config.toml` (gitignored), document in `config.toml.dist`.
- [ ] Decide tier slug. Spec uses `'paid'`. If marketing prefers `'pro'` or another label, change once now to avoid renaming later.

## Phase 1 — Schema, entitlement read path, dev CLI (no Stripe yet)

**Goal:** ground state for everything else. New table, new query, new dev tooling. App still has no upgrade flow, but the data layer is ready.

### Tasks
- [ ] sqlx migration: `subscriptions` table with the schema from the spec, plus `idx_subscriptions_user_active`.
- [ ] sqlx migration: `stripe_events` table (idempotency key store).
- [ ] Run `cargo sqlx prepare --workspace` after queries land; commit `.sqlx/` (memory: `feedback_sqlx_offline_cache.md`).
- [ ] `server/src/entity/subscription.rs`: `Subscription` struct + `Tier` enum (`Free`, `Paid`) + `Status` enum mirroring Stripe.
- [ ] `server/src/mappers/subscription_mapper.rs`: `find_active_for_user(user_id) -> Option<Subscription>`. The "is paid" query from the spec.
- [ ] `server/src/services/entitlement.rs`: thin service over the mapper exposing `effective_tier(user_id) -> Tier`. This is what middleware/handlers call.
- [ ] **CLI bin** `server/src/bin/set_subscription.rs`:
  - States: `free`, `trialing`, `active`, `past-due`, `cancel-at-period-end`, `canceled-expired`, `incomplete`.
  - Identify by `--user-id <int>` or `--email <addr>`.
  - Refuses to run if `config.app.environment == "production"`. Print friendly error and exit 2.
  - Excluded from the release Dockerfile build target (verify by checking `Dockerfile`).
- [ ] Backend tests:
  - Mapper test (sqlx::test) for each entitlement state → expected `Tier`.
  - CLI integration test: drive each state, assert resulting row shape.
  - Negative test: CLI refuses to run with `environment = "production"`.

### Acceptance
- `AWS_LC_SYS_PREBUILT_NASM=1 cargo build` clean.
- `cargo test -p server` — all green, including the new mapper + CLI tests.
- `cargo run --bin set_subscription -- --email <addr> --state active` updates the row; `--state free` removes it. Verify via `psql`.
- `cargo run --bin set_subscription -- --state active` against `environment = "production"` config exits with the safety guard message and code 2.

### Suggested commits
1. `feat(subscriptions): add subscriptions + stripe_events schema`
2. `feat(subscriptions): entitlement mapper + service`
3. `feat(dev): set_subscription CLI for entitlement testing`

---

## Phase 2 — Stripe Checkout + frontend pricing page (happy path only)

**Goal:** logged-in user can click "Start trial" → land on Stripe Checkout → return to a success or cancel screen. No webhook integration yet — the success page polls until the cron or Stripe tells us, but in this phase we'll cheat and mark the subscription via a return-side `checkout.session_id` lookup. **Webhook idempotency comes in Phase 3.**

### Tasks

#### Backend
- [ ] Add `async-stripe` dep to `server/Cargo.toml`. Pin the version. Feature flags: `runtime-tokio-hyper-rustls`, `webhook-events`, `checkout`, `billing-portal` (whichever the chosen version exposes).
- [ ] `config.toml.dist` + `Config` struct: `[stripe]` section (`publishable_key`, `secret_key`, `webhook_secret`, `price_id_monthly`, `price_id_annual`) + `[app]` section (`environment`, `frontend_url`).
- [ ] Inject Stripe client at startup as `web::Data<stripe::Client>`.
- [ ] `POST /api/stripe/checkout` (authed):
  - Accepts `{ "interval": "monthly" | "annual" }`.
  - Resolves the matching `price_id` from config.
  - Creates a `CheckoutSession` with the params from the spec (subscription mode, 14-day trial, card required, success/cancel URLs, `client_reference_id = user_id`).
  - If user already has a `stripe_customer_id` (re-subscribing after cancel), reuse it; else let Stripe create one.
  - Returns `{ "url": "<stripe_checkout_url>" }`.
- [ ] `GET /api/subscription/me` (authed): returns effective tier for the current user. Used by `/upgrade/success` polling.
- [ ] **Phase-2-only stopgap** for the success page: on `GET /api/subscription/me`, if the request includes a `?session_id=` query and we have no local row yet, fetch the session from Stripe and provisionally write the `subscriptions` row from the linked `subscription` object. Phase 3 webhooks will replace this stopgap.

#### Frontend
- [ ] New route `/upgrade` → `UpgradePage.vue`. Layout per `screens-upgrade.jsx` pricing surface:
  - Eyebrow + display heading + subtitle (i18n: `pricing.eyebrow`, `pricing.heading`, `pricing.subtitle`).
  - `UiSegmented` for monthly/annual toggle. Annual chip "2 months free".
  - Feature bullets list (i18n: `pricing.features.*`).
  - Primary CTA "Start 14-day free trial" → POST `/api/stripe/checkout` with selected interval → `window.location.href = response.url`.
  - Footer disclaimer (i18n: `pricing.footer`).
- [ ] New route `/upgrade/success` → `UpgradeSuccessPage.vue`. Reads `session_id` query, polls `GET /api/subscription/me?session_id=...` every 1s up to 10s. On tier flip, toast success + `router.replace('/my-calendars')`. On timeout, show "still processing" with a manual refresh CTA.
- [ ] New route `/upgrade/canceled` → `UpgradeCanceledPage.vue`. Light empty state + back link to `/upgrade`.
- [ ] All three routes are public-meta `false` (authed). Skip-link + h1 per route (Track 2 a11y conventions).
- [ ] i18n: add `upgrade.*` and `pricing.*` namespaces to `en.json` and `pt.json`.

### Acceptance
- Logged-in user navigates to `/upgrade`, picks monthly, clicks the CTA → lands on Stripe Checkout's hosted page (test mode).
- Pays with card `4242 4242 4242 4242` → returns to `/upgrade/success?session_id=...` → tier flips to `paid` within 10s → redirects to `/my-calendars`.
- Cancels checkout → returns to `/upgrade/canceled` → can navigate back.
- Frontend tests: pricing toggle interaction, success-page polling state machine, i18n key presence in both locales.
- `npm run lint && npm run test:unit && npm run build` clean.
- `cargo build && cargo test -p server` clean.

### Suggested commits
1. `feat(stripe): config + async-stripe client wiring`
2. `feat(stripe): POST /api/stripe/checkout for subscription sessions`
3. `feat(subscription): GET /api/subscription/me with session_id stopgap`
4. `feat(upgrade): pricing, success, canceled pages with monthly/annual toggle`
5. `feat(upgrade): i18n keys for pricing + upgrade namespaces`

---

## Phase 3 — Webhook endpoint + idempotent event handlers

**Goal:** Stripe's webhook events drive the local `subscriptions` table from now on. Drop the Phase-2 stopgap.

### Tasks
- [x] `POST /api/stripe/webhook` (public, no JWT): reads raw body + `Stripe-Signature` header, calls `stripe::Webhook::construct_event(body, sig, webhook_secret)` to verify. 400 on signature failure.
- [x] Whitelist `/api/stripe/webhook` from `actix-governor` rate-limiting (Stripe sends bursts on retries). *Implemented via custom `WebhookExemptKeyExtractor` in `server.rs` — maps the path to a sentinel-IP that's added to `whitelisted_keys`. Default per-peer-IP behavior preserved for everything else.*
- [x] Idempotency: insert `stripe_event_id` into `stripe_events` with `INSERT ... ON CONFLICT DO NOTHING RETURNING 1`. If 0 rows returned, return 200 immediately without further processing.
- [x] Multi-table writes (event + subscription) inside a single sqlx transaction with explicit rollback on error (CLAUDE.md rule).
- [x] Handler routes by `event.type`:
  - `checkout.session.completed`: deliberately a no-op — `customer.subscription.created` carries the same data plus our metadata, so we just record the event id and 200. *(Diverges from plan; cleaner since both events fire together and the subscription event has the full object inline.)*
  - `customer.subscription.created`: upsert from full payload, resolving `user_id` from `subscription.metadata.user_id` (stamped during Checkout creation) with customer→user fallback.
  - `customer.subscription.updated`: same path as `created` — `upsert_from_stripe_in_tx`.
  - `customer.subscription.deleted`: status → `canceled` via `update_status_by_subscription_id_in_tx`, row preserved for history.
  - `invoice.payment_succeeded`: update `current_period_end` from `lines.data[0].period.end`; flip `past_due` → `active` if previously dunning.
  - `invoice.payment_failed`: status → `past_due`.
  - All other events: idempotency insert + 200 (Stripe stops retrying).
- [x] Remove the Phase-2 `?session_id=` stopgap from `GET /api/subscription/me`. *Removed cleanly in this same Phase-3 landing rather than the staged "comment-then-delete" path, since the webhook + stopgap-removal land in one PR.*
- [ ] Backend tests: a fixture per event type (replayed payloads). For each event, assert resulting `subscriptions` row state. Replay the same event twice, assert no duplicate write (idempotency). *Not yet — only the `record_first_time` mapper has unit tests. Filed as follow-up.*

### Acceptance
- End-to-end test: complete a Stripe Checkout in test mode → webhook fires → `subscriptions` row appears. Verify via `psql`.
- Replay any webhook event from the Stripe Dashboard → handler is idempotent (no DB error, no row duplication).
- Bad signature → 400.
- Stopgap is gone; success page no longer relies on `?session_id`.

### Suggested commits
1. `feat(stripe): POST /api/stripe/webhook with signature verification`
2. `feat(stripe): idempotent event handler for subscription lifecycle`
3. `chore(subscription): remove Phase 2 session_id stopgap`

---

## Phase 4 — Customer Portal + Account → Subscription tab

**Goal:** paid users can self-manage. Cancel, switch billing interval, swap payment method — all hosted by Stripe; we just deep-link.

### Tasks
- [x] `POST /api/stripe/portal` (authed): looks up the user's `stripe_customer_id`. **Authorization check**: refuse if the looked-up customer-id doesn't belong to the calling user (defensive — should never trip, but documents intent). Creates a `BillingPortalSession` with `return_url = ${frontend_url}/account/subscription`. Returns `{ "url": "..." }`. *Defense in depth: customer-id is looked up server-side from JWT claims; no caller-supplied id to validate against, by design.*
- [x] New route `/account/subscription` and 5th `AccountPage` sidebar tab (`account.tabs.subscription`).
- [x] `SubscriptionTab.vue`:
  - **Free user**: empty state, "Upgrade" CTA → `/upgrade`.
  - **Paid user**: tier label, renewal/cancellation date, manage CTA → POST `/api/stripe/portal` → `window.location.href`.
  - **Past-due user**: same layout + danger banner.
  - **Trialing user**: same layout + info banner + trial-end date.
- [x] i18n: `account.subscription.*` keys in both locales.
- [x] Sidebar tab insertion: after Preferences, before Password.
- [x] Frontend tests: 11 cases covering each user state, manage-button POST, portal error, fetch error, locale parity.

### Acceptance
- Paid user visits `/account/subscription` → sees current state, clicks "Manage" → lands on Stripe Customer Portal → can cancel.
- Cancellation in the portal → webhook updates local `subscriptions.cancel_at_period_end = true` → reload `/account/subscription` shows "Cancels on <date>". Refresh after period_end → reverts to free state.
- 5 sidebar tabs render correctly (Profile, Preferences, Subscription, Password, Danger).

### Suggested commits
1. `feat(stripe): POST /api/stripe/portal for self-service`
2. `feat(account): subscription tab with state-aware UI`
3. `feat(account): wire subscription tab into AccountPage sidebar`

---

## Phase 5 — Pro accent enforcement ✅ Complete (2026-05-02)

**Goal:** the three Pro accents (Matcha, Sakura, Citron) become real entitlement-gated features. Free users get the interrupt modal; the server returns 402 if they bypass UI.

### Tasks
- [x] Frontend: list of Pro accents lives in one place (`frontend/src/constants/proAccents.ts`). Re-export from `AccentPicker`.
- [x] `AccentPicker.vue`: when free user clicks a Pro accent, prevent the click and emit an `interrupt` event instead of `select`. Emit existing event on free accents.
- [x] `PreferencesTab.vue`: handle `interrupt` by opening `<UpgradeInterruptModal>` (new component, uses `<UiModal>` so it inherits focus trap from the Track 2 fix).
- [x] `UpgradeInterruptModal.vue`: per `screens-upgrade.jsx` interrupt surface — headline, feature highlight, "See plans" primary CTA → `router.push('/upgrade')`, "Maybe later" ghost button. i18n: `interrupt.*`.
- [x] Backend: `PUT /user/settings` validates the requested `accent` against tier. Free user requesting a Pro accent → return `Error::PaymentRequired` (HTTP 402) with body `{"error":"upgrade_required","required_tier":"paid"}`.
- [x] New `Error::PaymentRequired` variant with the JSON body shape.
- [ ] Frontend axios interceptor or local handler: on 402 from `/user/settings`, surface the interrupt modal (defense in depth). _Deferred — UI gate already prevents this path; can revisit if telemetry shows API-only attempts._
- [x] Tier downgrade safety: `useTheme` exposes `setIsPaid` + `resolveAccent` — stored accent ref + localStorage are preserved, but the rendered DOM attribute falls back to default for free users. PreferencesTab calls `setIsPaid` after fetching subscription on mount.
- [x] Backend tests: PUT with Pro accent as free user → 402; as paid user → 200.
- [x] Frontend tests: interrupt modal opens on free-user Pro-click; not on paid-user click; downgrade scenario preserves stored accent but applies default.

### Acceptance
- Free user clicks Matcha → interrupt modal opens. Click "See plans" → lands on `/upgrade`.
- Paid user clicks Matcha → applies normally.
- Past-due user (still in grace period) → still allowed (per spec — they're paid until period_end).
- Free user POSTing Pro accent directly to `/user/settings` → 402 with the documented JSON body.
- Cancelling a Pro user's subscription past period_end → next reload, theme renders default accent; their saved preference is preserved on the row.

### Suggested commits
1. `feat(accent): centralize Pro accent list + free-user interrupt event`
2. `feat(upgrade): UpgradeInterruptModal + i18n strings`
3. `feat(api): 402 PaymentRequired on Pro accent for free users`
4. `feat(theme): downgrade-safe accent apply (preserve stored, render default)`

---

## Phase 6 — Reconcile loop + polish + e2e

**Goal:** safety net is live, copy is shipped, end-to-end matrix is validated against Stripe test mode.

### Tasks
- [ ] `server/src/services/reconcile.rs`: hourly `tokio::spawn` task per the spec.
  - Reads `[reconcile] interval_secs` from config (default 3600, 0 disables).
  - Iterates active rows, calls `stripe::Subscription::retrieve`, conditional UPDATE (Option α: `WHERE current_period_end <= $stripe_period_end`).
  - Emits `entitlement_reconcile_drift_total{field="..."}` Prometheus counter on every correction.
  - Per-row errors logged + skipped; pass continues.
- [ ] Spawn the loop from `main.rs` after Stripe client + DB pool are ready.
- [ ] Tests: with a `MockStripeClient` (or hand-rolled trait + fixture), seed local DB with stale data, run one pass, assert local matches mocked Stripe.
- [ ] Add `--reconcile-from-stripe` mode to the `set_subscription` CLI for manual single-user reconciliation during debugging.
- [ ] Final copy + design pass: pricing card visuals to match `screens-upgrade.jsx` precisely, including any banner-fade-in or hover effects from Track 1.
- [ ] e2e checklist (manual, against Stripe test mode):
  - [ ] Sign up → Checkout (monthly) → success → tier=paid.
  - [ ] Sign up → Checkout (annual) → success → tier=paid.
  - [ ] Card `4000 0000 0000 9995` (insufficient funds): trial signup succeeds, post-trial billing fails, `past_due`, banner appears.
  - [ ] Card `4000 0027 6000 3184` (SCA): authentication challenge surfaces in Checkout, completes after.
  - [ ] Manage Subscription → cancel → `cancel_at_period_end`, /account/subscription shows "Cancels on <date>".
  - [ ] Wait past period_end (or manually adjust via `set_subscription`) → tier reverts to free, Pro accents disabled.
  - [ ] Re-subscribe after cancellation → Stripe customer reused, new subscription row, tier=paid.
  - [ ] Webhook delivery loss simulation: temporarily reject webhooks, then re-enable; reconcile loop catches up at next tick.
- [ ] Stripe Dashboard production checklist:
  - [ ] Live-mode webhook endpoint registered.
  - [ ] Live prices created.
  - [ ] Stripe Tax configured (if charging EU users).
  - [ ] Customer Portal branding configured.

### Acceptance
- Reconcile loop runs hourly in dev; manually trigger drift (e.g. `UPDATE subscriptions SET current_period_end = '1970-01-01' WHERE ...`) → next pass corrects it; `entitlement_reconcile_drift_total` counter increments.
- All e2e checklist items pass against Stripe test mode.
- All tests + lint + build clean.
- Spec's open questions are resolved (or explicitly punted to a follow-up file).

### Suggested commits
1. `feat(stripe): hourly reconcile loop with drift metrics`
2. `feat(dev): set_subscription --reconcile-from-stripe mode`
3. `polish(upgrade): match design handoff visuals`
4. `docs(track-4): close out e2e checklist + production prep notes`

---

## After Phase 6

- Mark Track 4 complete in memory (`project_track_4_complete.md`) and the master breakdown.
- Track 3 (mobile companion) is the next sequenced track — a responsive pass over Tracks 2 + 4 surfaces.

## Risks tracked across phases

| Risk | Phase | Mitigation |
|---|---|---|
| Webhook delivery loss | 3 | Idempotency table + reconcile loop (Phase 6) + Stripe Dashboard manual replay. |
| Race between webhook + reconcile | 6 | Conditional UPDATE keyed on `current_period_end` (Option α). |
| User downgrade leaves them with an invalid Pro accent stored | 5 | Apply-time mapping in `useTheme`; preserve stored value. |
| `async-stripe` version drift / breaking changes | 1+ | Pin the version. Re-evaluate on Anthropic context-window-friendly schedule. |
| CLI accidentally run against prod | 1 | Two-layer guard: `environment != "production"` check + bin excluded from release Dockerfile. |
| Stripe webhook secret rotation | 3+ | Document the rotation runbook in `config.toml.dist` comments. Reconcile loop catches drift if webhooks silently break. |
| Multi-replica deploy makes reconcile loop racey | 6 (deferred) | Either `pg_try_advisory_lock` around the pass, or extract reconcile into its own service. Defer until multi-replica is real. |
