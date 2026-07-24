---
name: Stripe webhook user-id resolution via subscription metadata
description: Use subscription_data.metadata.user_id at Checkout time so subscription/invoice events carry the user mapping back without API roundtrips
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
**Rule:** When integrating Stripe webhooks, stamp `subscription_data.metadata.user_id` at Checkout creation time. Don't rely on `client_reference_id` for downstream mapping.

**Why:** `client_reference_id` only exists on the *original* Checkout Session object. Every downstream `customer.subscription.*` and `invoice.*` event payload arrives without it. Without metadata, the only ways to resolve the local user are: (a) fetch the original Checkout session via Stripe API on every webhook delivery, or (b) maintain a customer_id↔user_id lookup table seeded from the first event. (a) costs an API roundtrip + risks rate limits; (b) has a chicken-and-egg problem on the *first* `customer.subscription.created` event.

Stripe propagates `subscription_data.metadata` from Checkout → Subscription → every webhook payload that references that subscription, so a single stamp at the start carries forward indefinitely.

**How to apply:**
- In the Checkout creation request, set `subscription_data.metadata = {"user_id": "<id>"}`. With async-stripe-checkout 1.0.0-rc.5: `let mut sd = CreateCheckoutSessionSubscriptionData::new(); sd.metadata = Some(HashMap::from([("user_id".into(), user.id.to_string())]));`
- In the webhook handler, prefer `sub.metadata.get("user_id").and_then(|v| v.parse().ok())` over any other resolution path.
- Keep a customer→user fallback (`SubscriptionMapper::find_user_id_by_customer_in_tx` here) for out-of-band Stripe activity (dashboard-created subs, legacy data) — but don't rely on it as the primary path.
- Invoice events don't carry subscription metadata directly (only the subscription id). For those, the customer→user lookup is the right resolution path. By the time invoice events fire, the first `customer.subscription.created` will have populated the row.
