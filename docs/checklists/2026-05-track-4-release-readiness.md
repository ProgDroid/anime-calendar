# Track 4 Release Readiness — Stripe Upgrade Flow

**Date created:** 2026-05-03
**Owner:** _assign on first run_
**Source plan:** [`docs/superpowers/plans/2026-05-01-track-4-upgrade-flow.md`](../superpowers/plans/2026-05-01-track-4-upgrade-flow.md)

This is the runnable version of the Phase 6 deferred manual e2e + production prep
items. Each section maps to one user-visible flow. Tick the boxes as you go;
finish with the sign-off block at the bottom.

> **Read this first.** Run the checklist against **Stripe test mode** unless a
> section is explicitly labelled "Live mode". The reconcile loop runs hourly by
> default — set `[reconcile] interval_secs = 60` in `config.toml` for the
> webhook-loss simulation so you don't have to wait an hour. Restore it to
> `3600` when you're done.

---

## 0. Prerequisites

- [ ] `config.toml` has Stripe test secret + publishable + webhook-signing keys.
- [ ] Stripe Dashboard test-mode prices created: monthly + annual Pro.
- [ ] Test webhook endpoint registered in Stripe Dashboard pointing to your
      local tunnel (`stripe listen --forward-to ...` or equivalent).
- [ ] Local server running with `RUST_LOG=info,server=debug`. Frontend dev
      server running. A test user account is signed in.
- [ ] `set_subscription` binary built (`cargo build --bin set_subscription`).
- [ ] Prometheus scraping `/metrics` (or have `curl localhost:8080/metrics`
      ready) so you can verify `entitlement_*` counters.

---

## 1. Happy path — monthly subscription

- [ ] Sign up a fresh user → land on `/upgrade`.
- [ ] Click "Start free trial" with **Monthly** selected.
- [ ] Stripe Checkout opens. Pay with `4242 4242 4242 4242`, any future expiry,
      any CVC, any postal code.
- [ ] Lands on `/upgrade/success`. Toast appears. Page eventually shows the
      "Welcome to Pro!" body (webhook arrives within a few seconds).
- [ ] In `/account/subscription`, `status=trialing` (or `active` if your price
      has no trial), `current_period_end` is ~30 days out.
- [ ] DB row in `subscriptions` table:
      `tier='paid'`, `stripe_customer_id` is real, `stripe_subscription_id`
      is real, `stripe_price_id` matches the test monthly price.
- [ ] Pro accents (Matcha / Sakura / Citron) are now applyable from
      `/account` → Preferences without the interrupt modal.
- [ ] Webhook delivery: `checkout.session.completed` and
      `customer.subscription.created` (or `.updated`) both visible in the
      Stripe Dashboard, both responded with 200.

## 2. Happy path — annual subscription

- [ ] Same flow, but pick **Annual** on `/upgrade`.
- [ ] Stripe Checkout shows the annual price line.
- [ ] After payment, `/account/subscription` shows `current_period_end` ~365
      days out and the savings chip stays visible.

## 3. Insufficient funds (post-trial billing fail)

Card: `4000 0000 0000 9995`

- [ ] Sign up → Checkout → trial succeeds (no immediate charge).
- [ ] Fast-forward to post-trial billing. **Easiest path:** in Stripe
      Dashboard → Customers → your test customer → manually advance the test
      clock past trial end, or use `stripe trigger invoice.payment_failed`.
- [ ] Webhook arrives → DB updates to `status='past_due'`.
- [ ] `/account/subscription` shows the **past-due banner** (per Phase 4
      implementation), with a link to update payment method via the Customer
      Portal.
- [ ] Pro accents continue to work during grace period (per spec — they're
      paid until `current_period_end`).

## 4. SCA challenge (3DS)

Card: `4000 0027 6000 3184` — requires 3D Secure authentication on every
charge.

- [ ] Sign up → Checkout → enter the SCA card.
- [ ] Stripe surfaces the 3DS authentication challenge inside the Checkout
      session.
- [ ] Complete authentication → Checkout completes → land on success page.
- [ ] DB and entitlement state match the happy path (`tier='paid'`, status
      reflects trial/active per price config).
- [ ] _Optional:_ also test `4000 0027 6000 3184` with **declined**
      authentication — Checkout should keep the user on the form with an
      error, no DB row created, no `customer.subscription.created` webhook.

## 5. Cancel via Customer Portal

Continue from a `tier='paid'` user.

- [ ] `/account/subscription` → click **Manage subscription**.
- [ ] Stripe-hosted Customer Portal opens. Click **Cancel subscription**.
      Confirm.
- [ ] Webhook `customer.subscription.updated` arrives. DB row's
      `cancel_at_period_end` flips to `true`; `status` stays `active`.
- [ ] Back in `/account/subscription`, the page shows
      "**Cancels on `<date>`**" using the formatted period_end.
- [ ] The Stripe Dashboard subscription's "Cancel at period end" badge is set.

## 6. Tier reverts to free after period_end

You need to either wait, or shortcut this with the dev binary.

- [ ] **Shortcut:** stop the server, run
      `set_subscription --email <test-email> canceled-expired`,
      restart the server.
- [ ] On next page reload, `/account/subscription` shows free-tier UI.
- [ ] Pro accents are no longer applyable; clicking Matcha opens the
      `UpgradeInterruptModal`.
- [ ] If the user previously had a Pro accent set, the rendered accent in
      the DOM is the default (per Phase 5 downgrade-safe apply), but their
      stored preference is preserved in the DB row and `localStorage`.
- [ ] Backend test guard: `PUT /api/user/settings` with a Pro accent value
      now returns **402 PaymentRequired** with a JSON body matching the
      spec.

## 7. Re-subscribe after cancellation

- [ ] User from §6 (now `tier='free'`) starts another Checkout.
- [ ] Stripe **reuses the existing customer** — confirm in the Dashboard
      that no new `cus_...` was created.
- [ ] A new `subscription` row is created (or the existing one is re-used,
      per the upsert in `subscription_mapper`); `tier='paid'` is restored.
- [ ] Stored Pro accent (preserved through downgrade) is rendered again
      without the user touching settings.

## 8. Webhook delivery loss → reconcile catches up

This is the safety-net validation. Test against `[reconcile] interval_secs = 60`
so you don't wait an hour.

- [ ] Confirm reconcile loop is running:
      `curl -s localhost:8080/metrics | grep entitlement_reconcile`.
      You should see `entitlement_reconcile_drift_total` and
      `entitlement_reconcile_errors_total` with `0` values.
- [ ] **Disable webhook forwarding** (stop `stripe listen`, or temporarily
      remove the endpoint from the Stripe Dashboard).
- [ ] Cancel the subscription via the Customer Portal. The webhook should
      NOT reach your server, so the local DB row still says
      `cancel_at_period_end=false`.
- [ ] Wait one reconcile tick (~60s). The hourly loop fetches Stripe truth,
      sees the drift, applies the conditional UPDATE.
- [ ] `/metrics` now shows
      `entitlement_reconcile_drift_total{field="cancel_flag"}` incremented
      by ≥ 1.
- [ ] DB row's `cancel_at_period_end=true` matches Stripe.
- [ ] Re-enable webhook forwarding for subsequent runs.

**Manual single-user variant:** instead of waiting for the loop, run
`set_subscription --email <test-email> --reconcile-from-stripe` — it goes
through the same `reconcile_for_user` path and prints whether drift was
corrected.

## 9. Direct DB drift seed (smoke test the loop)

- [ ] Pick a paid test user. In `psql`, run:
      `UPDATE subscriptions SET current_period_end = '1970-01-01' WHERE user_id = <id>;`
- [ ] Wait one reconcile tick.
- [ ] Counter `entitlement_reconcile_drift_total{field="period"}` increments.
- [ ] DB row's `current_period_end` is back to Stripe's value.

## 10. Conditional-UPDATE guard (race protection)

- [ ] Pick a paid test user with a known `current_period_end` (e.g. 30 days
      from now).
- [ ] In `psql`, run:
      `UPDATE subscriptions SET current_period_end = current_period_end + INTERVAL '90 days' WHERE user_id = <id>;`
      (Simulates a webhook racing ahead.)
- [ ] Trigger `set_subscription --email <test-email> --reconcile-from-stripe`.
- [ ] Output: `no drift detected (local matches Stripe, or fresher)`. The
      cron's older Stripe snapshot did NOT regress the fresher local row.
- [ ] DB row's `current_period_end` unchanged (still +90 days).
- [ ] Reset by running `set_subscription --email <test-email> active`.

---

## 11. Stripe Dashboard — production prep

These are **live mode** items. Do them only when you're ready to flip the
switch.

- [ ] **Live-mode webhook endpoint** registered. Endpoint URL points to the
      production server. Subscribe to: `checkout.session.completed`,
      `customer.subscription.created`, `customer.subscription.updated`,
      `customer.subscription.deleted`, `invoice.payment_failed`,
      `invoice.payment_succeeded`. Live signing secret pasted into prod
      `config.toml`.
- [ ] **Live prices created** for monthly + annual Pro. Price ids pasted into
      prod `config.toml` under `[stripe.prices]`.
- [ ] **Stripe Tax** enabled if charging EU/UK customers. Origin address +
      tax registrations configured in Dashboard → Settings → Tax.
- [ ] **Customer Portal branding** set: Dashboard → Settings → Billing →
      Customer Portal. Choose what users can self-serve (cancel, switch
      interval, update payment method). Match the brand colors.

## 12. Production smoke test (post-deploy)

After the deploy, do a single end-to-end run with a real card on the live
site:

- [ ] Sign up a brand-new user (don't reuse a test account).
- [ ] Buy the monthly plan with a real card. Real charge, small amount.
- [ ] Verify webhook delivered, DB row created, `/account/subscription`
      reflects the right state.
- [ ] Cancel via Customer Portal. Confirm `cancel_at_period_end=true`.
- [ ] Issue a refund from the Stripe Dashboard (refund the test charge).
      Confirm the user's tier behaviour matches your refund policy (Pro
      until period end, then free).

---

## Sign-off

- [ ] All sections above completed (or explicitly skipped with a reason).
- [ ] `[reconcile] interval_secs` reset to `3600` after testing.
- [ ] Reconcile counter values captured for the team retrospective:
      - `entitlement_reconcile_drift_total{field="status"}`: ___
      - `entitlement_reconcile_drift_total{field="period"}`: ___
      - `entitlement_reconcile_drift_total{field="cancel_flag"}`: ___
      - `entitlement_reconcile_drift_total{field="trial"}`: ___
      - `entitlement_reconcile_errors_total`: ___
- [ ] Track 4 plan updated to mark Phase 6 deferred items as resolved.

**Run by:** ____________
**Date:** ____________
**Notes:** ____________
