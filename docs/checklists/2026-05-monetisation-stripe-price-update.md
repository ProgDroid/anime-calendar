# Monetisation Pricing — Stripe Price Update

**Date created:** 2026-05-04
**Owner:** _assign on first run_
**Source spec:** [`docs/superpowers/specs/2026-05-04-monetisation-refinement-design.md`](../superpowers/specs/2026-05-04-monetisation-refinement-design.md)
**Companion to:** [`2026-05-track-4-release-readiness.md`](./2026-05-track-4-release-readiness.md) (Track 4 covered the *first* Stripe rollout; this covers the *price change* that lands with the monetisation refinement spec.)

This checklist runs the Stripe-side configuration change for the new pricing introduced by the monetisation refinement work. It assumes the Track 4 readiness checklist has already been completed at least once — i.e., Stripe is wired in, webhooks deliver, the Customer Portal works, the reconcile loop is running.

> **Scope:** prices only. No product duplication, no schema changes, no new webhook events. The existing Pro product gets two new Price objects (monthly + annual), the old prices are archived, and `config.toml` is updated. Existing subscribers (none in prod yet — pre-release) would otherwise need a migration plan; that is out of scope until we have paying users.

> **Read this first.** Run sections 1–4 in **Stripe test mode**. Sections 5–6 are **live mode**. Use the new pricing values: monthly **$2.99**, annual **$24.99**, 14-day trial unchanged.

---

## 0. Prerequisites

- [ ] Track 4 readiness checklist has been completed at least once. Stripe webhooks deliver, Customer Portal works, reconcile loop is healthy.
- [ ] `config.toml` currently references the old test prices ($4.99 / $49.90). Note the existing price IDs before changing anything — you'll archive them after the new ones are live.
- [ ] Spec 1 (monetisation refinement) is implemented up to and including Phase 3 (pricing + UpgradePage rewrite + locale cleanup). Frontend pricing copy reads $2.99 / $24.99.
- [ ] Local server running with `RUST_LOG=info,server=debug`. Frontend dev server running. A test user account is signed in.
- [ ] `stripe listen --forward-to localhost:8080/api/stripe/webhook` running.

---

## 1. Create new test-mode prices

In **Stripe Dashboard → test mode → Products → Anime Calendar Pro** (the existing product — do **not** create a new product):

- [ ] Click **Add another price**. Pricing model: **Standard pricing**. Price: **$2.99 USD**. Billing period: **Monthly**. Trial period: **14 days**. Save.
- [ ] Click **Add another price** again. Pricing model: **Standard pricing**. Price: **$24.99 USD**. Billing period: **Yearly**. Trial period: **14 days**. Save.
- [ ] Copy both new `price_*` IDs into `config.toml` under `[stripe.prices]`:
      ```toml
      [stripe.prices]
      monthly = "price_NEW_MONTHLY_ID"
      annual  = "price_NEW_ANNUAL_ID"
      ```
- [ ] Restart the server to pick up the config change.
- [ ] **Do not archive the old prices yet** — they're still referenced by any in-flight test subscriptions. Archive after section 4.

## 2. Verify Checkout uses the new prices

- [ ] Sign up a fresh user → land on `/upgrade`.
- [ ] Confirm the page renders the new copy: "$2.99 / month", "$24.99 / year", "Save ~30%".
- [ ] Click **Start free trial** with **Monthly** selected. Stripe Checkout opens. **Verify the line item shows $2.99** with a 14-day free trial banner.
- [ ] Pay with `4242 4242 4242 4242`. Land on `/upgrade/success`. Webhook arrives.
- [ ] DB row in `subscriptions`: `tier='paid'`, `stripe_price_id` matches the **new** monthly price ID.
- [ ] Repeat with **Annual**. Verify Checkout shows $24.99 line item; DB row references the new annual price ID.

## 3. Verify the new prices flow through the Customer Portal

- [ ] From a `tier='paid'` user, click **Manage subscription** → Customer Portal opens.
- [ ] Portal "Plans" view shows monthly + annual at the **new** prices. Switching interval works and produces a `customer.subscription.updated` webhook.
- [ ] DB row's `stripe_price_id` updates to reflect the new selection.

## 4. Archive the old prices (test mode)

After §2 and §3 succeed:

- [ ] Stripe Dashboard → Products → Anime Calendar Pro → old monthly ($4.99) → click **⋯** → **Archive price**. Confirm.
- [ ] Same for the old annual ($49.90).
- [ ] Confirm Checkout still works (it should — `config.toml` only references the new IDs now).
- [ ] Any subscriptions still pointing at the old prices remain valid (Stripe never auto-migrates — archiving only prevents *new* subscriptions on the price). For pre-release / test mode, this doesn't matter; you can delete the test users.

---

## 5. Live-mode price creation

Do this when Phase 3 of spec 1 is ready to deploy.

- [ ] Stripe Dashboard → **switch to live mode** → Products → Anime Calendar Pro.
- [ ] Add the **$2.99 monthly** price with 14-day trial (same shape as §1).
- [ ] Add the **$24.99 annual** price with 14-day trial.
- [ ] Copy live `price_*` IDs into the **production** `config.toml`. Do **not** commit live IDs to git — use the deployment secret store.
- [ ] If old live prices exist (they shouldn't pre-release, but if you've done any soft-launch billing): archive them after the deploy.
- [ ] **Stripe Tax** — if you've turned this on per the Track 4 checklist §11, confirm the new prices inherit the same tax behaviour. Newly-created prices default to "inclusive of tax" per product setting; verify in the Dashboard.

## 6. Production smoke test (post-deploy)

- [ ] Sign up a brand-new user on the production frontend. Confirm `/upgrade` shows $2.99 / $24.99.
- [ ] Buy monthly with a real card (small charge). Verify success page, webhook, DB.
- [ ] Verify `/account/subscription` shows the new pricing in the "Renews on" line.
- [ ] Cancel via Customer Portal. Confirm `cancel_at_period_end=true`.
- [ ] _Optional:_ refund the charge if your refund policy permits, then verify the user's tier transitions correctly (Pro until period end, then free).

---

## Sign-off

- [ ] All sections above completed (or explicitly skipped with a reason).
- [ ] Old test-mode prices archived.
- [ ] Live-mode prices in place; production `config.toml` references the live IDs.
- [ ] Frontend pricing copy in `en.json` and `pt.json` confirmed at $2.99 / $24.99.
- [ ] No reports of confused users on the old pricing (search recent support / error logs).

**Run by:** ____________
**Date:** ____________
**Notes:** ____________
