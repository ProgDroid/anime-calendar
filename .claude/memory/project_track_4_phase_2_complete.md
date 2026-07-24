---
name: Track 4 Phase 2 complete (2026-05-02)
description: Stripe Checkout integration shipped — backend POST /api/stripe/checkout + GET /api/subscription/me + 3 frontend routes
type: project
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Track 4 (Stripe upgrade flow) Phase 2 landed 2026-05-02. The happy path is end-to-end functional against Stripe test mode.

**Why:** Stripe-hosted Checkout was the lowest-friction entry point — Phase 2 deliberately skipped the webhook so the contract between frontend, Stripe, and the backfill stopgap could be validated in isolation before adding inbound idempotency in Phase 3.

**How to apply:**
- Phase 3 (webhooks) is the next step; the `?session_id=` stopgap in `GET /api/subscription/me` is the explicit removal target — `controllers/subscription.rs::backfill_from_session` should be deleted once the webhook handler is canonical.
- async-stripe is pinned to `=1.0.0-rc.5` modular crates (checkout, billing, core, shared, types, webhook). Don't drift versions across the sub-crates — they all need to match.
- New mapper methods on `SubscriptionMapper`: `find_latest_customer_id_for_user` (Stripe customer reuse on re-subscribe) and `upsert_from_stripe` (canonical writer used by both stopgap and the upcoming webhook).
- Frontend polling: 10 attempts × 1s on `/upgrade/success`. The success page forwards `?session_id` to the read endpoint; backend matches `client_reference_id` against the caller user_id before writing.
