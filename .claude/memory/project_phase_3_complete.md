---
name: Phase 3 audit remediation complete (2026-05-07)
description: 6 audit findings (CD-LICENSE-1, C-1, H-4, H-6, H-7, H-8, H-10) shipped on audit/phase-3-critical, merged to main; 12 commits
type: project
originSessionId: d141fb2f-b707-4ff0-8ba3-228ad77d407f
---
`audit/phase-3-critical` merged to local main 2026-05-07. 6 audit findings closed across 12 commits (one was a typecheck-fix follow-up to T2):

- **CD-LICENSE-1 (HIGH license)** — actix-governor 0.7.0 (GPL-3.0) replaced with in-house MIT actix middleware wrapping the MIT `governor` crate. Lives at `server/src/middleware/rate_limit.rs` (~327 lines). Same 60-burst/1-rps semantics, `/stripe/webhook` exempted, byte-identical 429 body. `deny.toml` exception removed.
- **C-1 (CRITICAL)** — `delete_user` now opens a tx, checks `find_active_for_user_with`, returns `409 Conflict {"error":"active_subscription"}` if found. Frontend `DangerZoneTab` catches → renders Stripe Portal modal. New i18n keys under `account.danger.activeSubscription.*` (en + pt).
- **H-7 + H-8** — `apply_reconcile_update` WHERE clause tightened to also reject equal-period status regressions. New webhook handlers for `customer.subscription.paused`, `customer.subscription.resumed`, `customer.deleted`, `invoice.payment_action_required`. New mapper `update_all_status_by_customer_in_tx`. New metric `STRIPE_WEBHOOK_PAYMENT_ACTION_REQUIRED_TOTAL`.
- **H-4** — `safe_url` extracted to `redact_path` free fn with 5 unit tests; extended to scrub `/invitations/{token}[/action]`. nginx adds `^~ /invite/` and `^~ /api/invitations/` location blocks with `access_log off;`.
- **H-6** — New `SseConnectionTracker` service (`Mutex<HashMap<i32, usize>>` behind `Arc`). Default cap 8, configurable via `SharingConfig.sse_max_connections_per_user`. Returns `429` past cap. RAII guard moved INTO `async_stream::stream!` block.
- **H-10** — Invite token no longer in `?redirect=/invite/{token}` URL query. New `composables/inviteRedirect.ts` (stash + consume-once via sessionStorage). InviteLandingPage's 3 router-links became `<button>`s.

Final gates: 358 server lib tests / 479 frontend tests / build / lint all green.

Outstanding Phase 4+ docket: H-3 advisory-lock bypass on `add_item`, H-12 through H-18 (Quality & DRY), CR-P2-1/2/3 from Phase 2 cumulative review, 13 deferred RUSTSEC advisories in `deny.toml`.

Branch was a fast-forward to main; `audit/phase-3-critical` can now be deleted. Local main is ahead of `origin/main` by Phase 1+2+3 commits — push when ready.
