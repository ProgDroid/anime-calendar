---
name: Track 4 Phase 4 complete (2026-05-02)
description: Stripe Customer Portal endpoint + SubscriptionTab in Account; paid users can self-manage cancellations and payment methods
type: project
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Track 4 (Stripe upgrade flow) Phase 4 landed 2026-05-02, same day as Phases 1-3.

**Why:** Once paid users existed (Phases 1-3 covered subscribe + maintain state), they needed a way to cancel and update billing without contacting support. Stripe's Customer Portal is hosted, PCI-scope-zero, and handles cancellation/swap-card/swap-interval out of the box — we just deep-link.

**How to apply:**
- `POST /api/stripe/portal` lives at `controllers/stripe.rs::create_portal_session`. Defense in depth: customer-id is resolved server-side from JWT claims via `find_latest_customer_id_for_user` — there's no caller-supplied id to validate. Free users (no customer row) get 400 with `Error::InvalidRequest`. Stripe API failures bubble as 500 with `Error::Stripe(e.to_string())`.
- Frontend service: `frontend/src/services/subscription.ts::createPortalSession` — POST with no body, returns `{url}`. Caller does `window.location.href = url`. After Stripe sends user back, they land at `/account/subscription` (configured via `return_url`).
- `SubscriptionTab.vue` is a 5-state surface: free / paid / past_due / trialing / canceled. The state computed comes off `entitlement.tier + entitlement.status`; the webhook handler is the source of truth so we never second-guess.
- 5 tabs in `AccountPage` sidebar (Profile, Preferences, **Subscription**, Password, Danger). Insertion order matters — billing groups with prefs, danger stays last.
- Phase 5 (Pro accent enforcement) is next: gate the 4 Pro accents behind tier check, surface UpgradeInterruptModal when free users click them, return 402 from `PUT /user/settings`.

**Test counts after Phase 4**: backend 170 + 1 integration (unchanged), frontend 289 (was 276 → +11 from SubscriptionTab.spec, plus the 2 AccountPage tests modified to expect 5 tabs not 4).
