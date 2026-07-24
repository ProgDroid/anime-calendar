---
name: Billing endpoints — resolve identity from claims, accept no caller-supplied id
description: For self-service billing endpoints (portal, cancel-my-sub), look up the target id (stripe_customer_id, sub_id) server-side from JWT claims. Never accept it from the request body.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
**Rule:** Self-service billing endpoints (`POST /api/stripe/portal`, future cancel/upgrade endpoints) must resolve the target Stripe id server-side from JWT claims. Don't accept `stripe_customer_id` or `subscription_id` from the request body, even with an "is this yours?" check.

**Why:** "Validate the supplied id matches the user" is a defensible pattern, but the simpler shape is "have no supplied id at all." With no caller-input id:
- Nothing to forge in the request body.
- Nothing to validate against — the lookup IS the authorization.
- No path where a buggy validator (off-by-one in a comparator, type coercion, race) lets through a wrong id.
- The endpoint signature itself documents the trust model: `POST /api/stripe/portal` (no body) ↔ "act on the caller's billing relationship."

This came up in Track 4 Phase 4. The plan originally suggested an authz check ("refuse if the customer id doesn't belong to the calling user"), and we collapsed that into "look it up from claims, period" — same security outcome, smaller surface.

**How to apply:**
- Endpoint takes no request body (or only billing-mode-style enums like `interval: monthly|annual` that don't identify a specific customer/sub).
- Use `find_latest_customer_id_for_user(claims.user_id)` (or equivalent) inside the handler.
- Empty result → 400 `Error::InvalidRequest` (free user with no Stripe customer hitting the portal endpoint). The frontend should gate the CTA so this rarely trips, but the 400 documents intent for direct API callers.
- Caveat: this only applies to *self-service* endpoints. Admin endpoints (an admin acting on a specific user) are a different shape — they need explicit target identity in the URL path or body, plus admin-role authz.
