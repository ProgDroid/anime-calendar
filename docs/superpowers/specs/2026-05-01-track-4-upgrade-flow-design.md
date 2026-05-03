# Track 4 — Upgrade Flow (Pro tier + Stripe)

**Date:** 2026-05-01
**Status:** Spec / brainstormed (pending implementation plan)
**Predecessor tracks:** Track 1 (foundations) ✅, Track 2 (existing-surfaces re-skin + a11y) ✅
**Master:** `2026-04-30-design-redesign-master-breakdown.md` (Track 4)
**Design source:** `design_handoff_anime_calendar/screens-upgrade.jsx` + paywall surfaces inside `screens-extras.jsx`.

## Goals

1. Introduce a single paid tier (provisionally `paid`) alongside `free`, with room to add a third tier later without schema changes.
2. Wire Stripe Checkout for monthly + annual billing (2-months-free annual discount = 16.66%, equivalent to 10× monthly price).
3. 14-day free trial, **card required upfront** at checkout.
4. Enforce Pro gating on Matcha / Sakura / Citron accents (currently visual-only chip; flip to real entitlement enforcement).
5. Ship a developer CLI for setting any user's subscription state in test environments — production-locked.
6. Use Stripe Customer Portal for user-facing sub management (cancel, swap card, switch billing interval). No custom billing UI.

## Non-goals (explicit)

- **Per-feature granularity beyond accents.** Track 4 gates the three Pro accents. Anything else (calendar count limits, export limits, etc.) is deferred until the product asks for it.
- **Multi-currency.** USD only at v1.
- **Tax handling.** Use Stripe Tax (handled in Stripe dashboard); no app-side tax math.
- **Refund / proration UX.** Stripe Customer Portal handles cancellation + downgrade at period end. No in-app refund flow.
- **Promo codes.** Skip for v1; Stripe Coupons can be added later via dashboard without code changes.
- **Mobile companion.** Track 3 will responsive-pass these screens later.

## Architecture

### Entitlement model — Option B

Single new table `subscriptions`. A user is "paid" iff they have a row whose `(status, current_period_end)` represent live access.

```sql
CREATE TABLE subscriptions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier VARCHAR(32) NOT NULL,                              -- 'paid' (future: 'studio', etc.)
    status VARCHAR(32) NOT NULL,                            -- mirrors Stripe: 'trialing','active','past_due','canceled','incomplete','unpaid'
    stripe_customer_id VARCHAR(255) NOT NULL,
    stripe_subscription_id VARCHAR(255) NOT NULL UNIQUE,
    stripe_price_id VARCHAR(255) NOT NULL,                  -- monthly or annual price
    current_period_start TIMESTAMPTZ NOT NULL,
    current_period_end TIMESTAMPTZ NOT NULL,
    trial_end TIMESTAMPTZ,                                  -- NULL outside trial
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscriptions_user_active
    ON subscriptions(user_id, status, current_period_end);

CREATE TABLE stripe_events (
    stripe_event_id VARCHAR(255) PRIMARY KEY,               -- idempotency key
    event_type VARCHAR(64) NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Effective entitlement query** (used by middleware):

```sql
SELECT tier, status, current_period_end, cancel_at_period_end, trial_end
FROM subscriptions
WHERE user_id = $1
  AND status IN ('trialing','active','past_due')
  AND current_period_end > NOW()
ORDER BY current_period_end DESC
LIMIT 1;
```

If no row → `free`. Otherwise → `paid` with the returned metadata.

**No Redis caching for v1.** Indexed query on a small per-user table is sub-ms. Avoids the cache-invalidation complexity that has bitten this codebase before (see `feedback_cache_invalidation.md`). Add caching later only if Prometheus shows the query as hot.

### Stripe integration

- **SDK:** `async-stripe` crate (well-maintained, async, integrates with reqwest/tokio). Pin to a specific version in `Cargo.toml`.
- **Mode-aware config:** `config.toml` carries `stripe.publishable_key`, `stripe.secret_key`, `stripe.webhook_secret`, `stripe.price_id_monthly`, `stripe.price_id_annual`. Test-mode keys in dev/staging, live-mode in prod.
- **Checkout session** created server-side with:
  - `mode: 'subscription'`
  - `line_items: [{ price: <chosen price_id>, quantity: 1 }]`
  - `subscription_data: { trial_period_days: 14 }`
  - `payment_method_collection: 'always'` (card required during trial signup)
  - `success_url: ${frontend_url}/upgrade/success?session_id={CHECKOUT_SESSION_ID}`
  - `cancel_url: ${frontend_url}/upgrade/canceled`
  - `client_reference_id: user_id` (so webhook can correlate even before customer-id linkage exists)
  - `customer: <cached stripe_customer_id>` if user already has one; else Stripe creates one on first checkout.
- **Customer Portal session** created with `customer: stripe_customer_id, return_url: ${frontend_url}/account/subscription`.

### Reconcile loop (webhook safety net)

In-process `tokio` task spawned from `main.rs`, runs hourly. Webhooks remain primary (sub-second upgrade unlock); cron is the safety net that catches whatever the webhook path missed.

Each pass:
1. `SELECT * FROM subscriptions WHERE status NOT IN ('canceled') ORDER BY updated_at ASC`.
2. For each row, `stripe::Subscription::retrieve(stripe_subscription_id)`.
3. If `(status, current_period_end, cancel_at_period_end, trial_end)` differ from local, **conditionally** update — Option α: `UPDATE subscriptions SET ... WHERE id = $1 AND current_period_end <= $stripe_period_end`. The guard prevents the cron from regressing a fresher webhook write that landed mid-pass.
4. Increment `entitlement_reconcile_drift_total{field="status|period|cancel_flag|trial"}` Prometheus counter on every correction; log a warning so reliability incidents are traceable.
5. Per-row errors are caught and logged; the pass continues to the next row.

Config:
```toml
[reconcile]
interval_secs = 3600    # hourly. Set to 0 to disable (used in tests).
```

Frequency: hourly. Stripe's default API rate limit is 100 reads/sec; we'd need 360,000 active subs before this cron used 1% of that budget.

**Multi-replica note (deferred).** This loop is single-process safe. If the deployment scales to multiple replicas, the simple fix is `pg_try_advisory_lock` around each pass — only the lock-holder runs. A cleaner alternative is to extract reconcile into its own deployable service / k8s CronJob; that keeps the entitlement code path free of leader-election plumbing and lets the schedule live in deploy config. Pick whichever fits the deploy story at that point. Not a v1 concern.

**Manual force-reconcile** is reachable through the `set_subscription` dev CLI by adding a `--reconcile-from-stripe` mode (fetches Stripe truth, writes locally). No HTTP admin endpoint needed.

### Webhook handling

Endpoint: `POST /api/stripe/webhook` — public route (Stripe-signed, not JWT-authed).

Verify Stripe signature using `stripe::Webhook::construct_event` with the configured webhook secret. Reject any request that fails signature verification with 400.

Idempotency: every event's `id` is inserted into `stripe_events`; if `INSERT ... ON CONFLICT DO NOTHING` returns 0 rows, the event was already processed — return 200 immediately.

Events handled:

| Event | Action |
|---|---|
| `checkout.session.completed` | Read `client_reference_id`, link `stripe_customer_id` and `stripe_subscription_id` to user. Create initial `subscriptions` row. |
| `customer.subscription.created` | Upsert `subscriptions` row (defensive; Stripe sometimes fires this before checkout.session.completed). |
| `customer.subscription.updated` | Update status, period boundaries, cancel_at_period_end, trial_end. Most renewal/cancellation flows surface here. |
| `customer.subscription.deleted` | Set status to `canceled` (final). |
| `invoice.payment_succeeded` | Update `current_period_end` (renewal). |
| `invoice.payment_failed` | Status flips to `past_due`. UI shows banner. |

All other events are accepted (200) and ignored after the idempotency-key insert. This guarantees Stripe doesn't retry.

**Multi-table mutation rule** (per CLAUDE.md): every webhook handler that touches both `stripe_events` and `subscriptions` runs in a single sqlx transaction with explicit rollback on error.

### Entitlement middleware

New `EntitlementMiddleware` (or extension) attaches to authed routes that need gating. Reads from `subscriptions` once per request, exposes a `Tier` extractor.

For v1, gating is **only** in the accent-validation flow (PUT settings + accent reconcile). The middleware is implemented as a service helper rather than a global middleware to keep the door open for per-feature use later without re-plumbing.

### Frontend surfaces

Per the design handoff (`screens-upgrade.jsx`), four screens:

1. **Interrupt** — modal over the current page when a free user attempts a Pro action (selecting a Pro accent, etc.). Headline + subtitle + "See plans" CTA.
2. **Pricing / Paywall** — `/upgrade` route. Monthly/annual toggle (`UiSegmented`), feature list, "Start 14-day trial" CTA → POST `/api/stripe/checkout` → redirect to Stripe-hosted page.
3. **Success** — `/upgrade/success` route. Reads `session_id` from query, polls `/api/subscription/me` until tier flips (covers the small webhook latency window). Toast + redirect to `/my-calendars` after confirmation.
4. **Canceled** — `/upgrade/canceled` route. Light empty state + back link to `/upgrade`.

Plus an **Account → Subscription** tab (5th account tab, fits the existing sidebar pattern):
- Free user: shows current tier + "Upgrade" link to `/upgrade`.
- Paid user: shows tier, billing interval, next renewal date or cancel date, and "Manage subscription" button → POST `/api/stripe/portal` → redirect to Stripe Customer Portal.

### Pro accent enforcement

Currently `PreferencesTab.vue`'s AccentPicker shows the `Pro` chip on Matcha/Sakura/Citron but doesn't gate selection. Flip to:
- Free user clicks Matcha/Sakura/Citron → opens **Interrupt** modal (screen 1) instead of selecting.
- Paid user (active or trialing) → selection works as today.
- Past-due paid user (grace period) → still allowed; UI shows a payment-failure banner separately.
- Server-side `PUT /user/settings` rejects non-default accents from free users with `Error::PaymentRequired` (HTTP 402). Always-200 anti-enumeration rule does not apply here — this isn't an auth probe.

### Developer CLI

Cargo bin: `server/src/bin/set_subscription.rs`. Built only via `cargo run --bin set_subscription`; not part of the release artifact.

```
cargo run --bin set_subscription -- --email user@example.com --state active
cargo run --bin set_subscription -- --user-id 42 --state trialing
cargo run --bin set_subscription -- --email u@x.com --state cancel-at-period-end
```

States: `free`, `trialing`, `active`, `past-due`, `cancel-at-period-end`, `canceled-expired`, `incomplete`.

**Safety:** binary refuses to run unless the loaded `config.toml` has `app.environment != "production"`. No env-var override. The bin source is excluded from the release Dockerfile build target. Two layered defenses, intentional.

### Configuration changes

`config.toml.dist` gains:

```toml
[stripe]
publishable_key = ""        # pk_test_... in dev, pk_live_... in prod
secret_key = ""             # sk_test_... in dev (committed-blank in dist)
webhook_secret = ""         # whsec_... from Stripe Dashboard
price_id_monthly = ""       # price_... for the paid tier monthly rate
price_id_annual = ""        # price_... for the paid tier annual rate

[app]
environment = "development" # 'development' | 'staging' | 'production'
frontend_url = "http://localhost:5175"
```

~~`VITE_STRIPE_PUBLISHABLE_KEY` baked at build time per the existing Vite/Docker pattern.~~ **Not implemented this way.** Stripe Checkout is fully server-side: the backend creates a Checkout Session and returns the redirect URL, so the frontend never embeds the Stripe publishable key and no `VITE_STRIPE_*` env var was ever introduced. (Note 2026-05-03: even if a publishable key were needed in future, it would now be served via `/api/public-config` rather than baked at build time — see `feedback_vite_docker_env_vars.md` for the runtime-config pattern.)

### Security

- **Webhook signature verification** is mandatory. Reject unverified payloads with 400 before any DB work.
- **Stripe secret key** lives only in server config. Never in frontend bundles.
- **Customer Portal session** must verify the calling user's `stripe_customer_id` matches the requested portal target — prevents user A from generating a portal link for user B.
- **CSRF**: Stripe-bound endpoints (`/api/stripe/checkout`, `/api/stripe/portal`) require an authenticated session (httpOnly cookie). The webhook endpoint is exempt because it's signature-verified.
- **PCI scope**: zero. Card details enter Stripe Checkout's hosted page, never our server. We store `stripe_customer_id` strings only.
- **Rate limiting**: existing `actix-governor` covers all routes. Stripe webhook endpoint is whitelisted from the rate-limiter (Stripe sends bursts on retries).

### i18n

All new strings land in `en.json` and `pt.json` under namespaces:
- `upgrade.*` for /upgrade, /upgrade/success, /upgrade/canceled
- `interrupt.*` for the Pro-gating modal
- `account.subscription.*` for the new tab
- `pricing.*` for the pricing card content (feature list, interval toggle, CTAs)

### Testing strategy

- **Backend unit:** webhook handler with replayed Stripe event payload fixtures (use `async-stripe`'s test helpers). Cover idempotency: same event posted twice = single insert.
- **Backend integration:** real Postgres + mocked `async-stripe` client; assert `subscriptions` row state transitions on each event type.
- **Frontend unit (vitest):** pricing-toggle interactions, interrupt modal trap inheritance from UiModal, /upgrade/success polling state machine.
- **End-to-end:** Stripe test-mode card numbers (4242, 4000-0000-0000-9995 for declines, 4000-0027-6000-3184 for SCA challenges). Manual run, not automated, since Stripe's test mode rate-limits CI.
- **CLI:** integration test sets each state, confirms `subscriptions` row matches expected shape.

## Sequencing & risk

Implementation phases — see `2026-05-01-track-4-upgrade-flow.md` (plan) for the breakdown. Headline order:

1. Schema + entitlement read path + CLI (no Stripe yet — pure data).
2. Stripe Checkout creation endpoint + frontend pricing page (happy path only).
3. Webhook endpoint + event handlers (idempotent, all 6 events).
4. Customer Portal + Account → Subscription tab.
5. Pro accent enforcement (interrupt modal + server-side 402).
6. Polish, copy, e2e in Stripe test mode.

**Risk: webhook delivery loss.** Mitigation: idempotency table + Stripe Dashboard's manual replay UI **plus an in-process hourly reconcile loop** (see "Reconcile loop" section below) that catches drift between Stripe's truth and our local `subscriptions` table.

**Risk: trial→active transition timing.** Stripe fires `customer.subscription.updated` when the trial converts. Webhook handler must update `status` from `trialing` to `active`. Covered.

**Risk: clock skew between Stripe and our DB on `current_period_end`.** Stripe is source of truth; we store what they tell us. Don't compute period boundaries locally.

## Open questions before implementation

None remaining for the brainstorm. The plan can proceed.

Final placeholder values to fill before launch (not blockers for implementation):
- Actual price points (e.g., $4.99/mo, $49.99/yr).
- Exact pricing-card copy (lead, feature bullets, footer disclaimer).
- Stripe Tax setup in Dashboard.
