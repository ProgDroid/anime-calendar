---
name: Track 4 Phase 3 complete (2026-05-02)
description: Stripe webhook handler shipped — POST /api/stripe/webhook with signature verification, idempotency, and atomic event/subscription writes
type: project
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Track 4 (Stripe upgrade flow) Phase 3 landed 2026-05-02, same day as Phase 2.

**Why:** Phase 2's `?session_id=` stopgap was a polling-after-redirect kludge — fine for validating the contract end-to-end but not durable. Phase 3 wires the canonical webhook so subscription state in our DB tracks Stripe via push, not pull.

**How to apply:**
- Webhook lives at `server/src/controllers/stripe_webhook.rs`. Signature verification via `stripe_webhook::Webhook::construct_event`. Bad signature → 400 (Stripe disables endpoints on sustained 4xx, which is correct if the secret is wrong).
- Idempotency table is `stripe_events` (PK on `stripe_event_id`). The mapper has a `_in_tx` static helper so the idempotency insert and the business write share one transaction — rollback on handler failure means the event id is gone, so a Stripe retry will reprocess.
- **User-id resolution**: subscription metadata `user_id` is stamped during Checkout creation (`subscription_data.metadata`). Every `customer.subscription.*` event payload carries it back, so we never need to fetch the original Checkout session. Fallback: customer→user lookup against prior subscription rows.
- **Rate-limiting**: actix-governor's per-IP key extractor was replaced with a custom `WebhookExemptKeyExtractor` (`server/src/server.rs`). Maps `/stripe/webhook` to `0.0.0.0` sentinel IP that's in `whitelisted_keys`. All other paths preserve the original PeerIp + IPv6 /56-prefix bucketing.
- **`checkout.session.completed` is a no-op**: deliberately. Stripe fires `customer.subscription.created` alongside it with the full subscription object + our metadata, so writing twice would be redundant. Recording the event id is enough to stop retries.
- **Phase-2 stopgap removed in this same landing** — `controllers/subscription.rs::backfill_from_session` and `SubscriptionMeQuery` are gone. The `GET /api/subscription/me` endpoint is read-only now. Frontend can keep sending `?session_id=` (axios just ignores unknown params); no frontend change needed.
- **Webhook end-to-end test coverage missing**: only the idempotency mapper has unit tests. Filed as a follow-up — fixture-based event payloads for the 5 routed event types, assert resulting subscriptions-row state, then replay each twice for idempotency.
- Phase 4 (Customer Portal + Account → Subscription tab) is the next phase per the plan.
