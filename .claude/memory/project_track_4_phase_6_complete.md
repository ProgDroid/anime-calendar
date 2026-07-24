---
name: Track 4 Phase 6 complete + deferred items closed (2026-05-03)
description: Hourly reconcile loop + drift metrics + CLI reconcile mode shipped 2026-05-02. Deferred pricing-card visual polish + e2e checklist closed 2026-05-03. Track 4 is now fully done end-to-end.
type: project
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Phase 6 reliability work landed 2026-05-02:

1. `server/src/services/reconcile.rs` — hourly `tokio::spawn` task. Generic `StripeSubscriptionFetcher` trait (native AFIT, no async-trait dep) lets tests swap in `MockFetcher` without spinning a real Stripe client. `RemoteSubscription` is a slim projection (status, period_end, cancel_at_period_end, trial_end) so the live + mock impls stay narrow. `LiveStripeFetcher` wraps `stripe::Client` and uses `RetrieveSubscription::new(id).send(&client)`.
2. **Conditional UPDATE guard** (Option α): `apply_reconcile_update` runs `UPDATE subscriptions SET ... WHERE id = $1 AND current_period_end <= $stripe_period_end`. rows_affected = 0 means a webhook write landed mid-pass and moved local past Stripe's snapshot — safe outcome, no regression.
3. **Drift counter**: `entitlement_reconcile_drift_total{field="status|period|cancel_flag|trial"}` increments per drift detected and successfully applied. `entitlement_reconcile_errors_total` for per-row transport/parse failures.
4. **`[reconcile] interval_secs`** config (default 3600, 0 disables). Loop is also gated on `StripeConfig::is_configured()` — no point hitting Stripe with an empty secret key.
5. **`set_subscription --reconcile-from-stripe`** CLI mode reuses `reconcile_for_user(mapper, fetcher, user_id)` for manual single-user runs against Stripe truth. Loads config.toml for the secret key. Existing prod-env guard still applies.

**Why:** webhooks remain the primary path (sub-second upgrade unlock) but at-least-once delivery + Stripe Dashboard replay are not enough on their own. The reconcile loop is the safety net that catches whatever the webhook missed, and the conditional guard is what makes it safe to run against a live system.

**Coverage:** 7 reconcile tests (status drift, period drift, race-with-newer-local, no-drift, canceled-skipped, ghost-row continuation, pure `collect_drift` unit test). Total server suite is 179 tests.

**Deferred items — closed 2026-05-03:**
- ✅ Pricing-card visual polish (`796a44b`): replaced single-card layout with the design's 2-tier comparison (Free + Pro), centered hero with `<i18n-t>`-driven italic emphasis word, atmosphere radial gradient, elevated Pro card with accent border + `0 12px 40px var(--accent-1-glow)` shadow + "★ Most popular" chip, hover lift on lg+ via `--d-3`/`--ease-out`. Studio tier intentionally omitted (no Stripe price exists; design has 3 tiers but product sells only Pro). New i18n keys under `pricing.tiers.{free,pro}` + `pricing.mostPopular` + `pricing.headingEmphasis`. UpgradePage tests grew from 6 → 7 (added Free-card-disabled + Most-popular-chip assertion). 294 frontend tests pass.
- ✅ E2E checklist (`8048ba5`): runnable doc at `docs/checklists/2026-05-track-4-release-readiness.md`. 12 sections covering monthly/annual happy paths, insufficient funds (4000…9995), SCA (4000…3184), cancel via Customer Portal, tier reversion, re-subscribe, webhook-loss → reconcile catch-up (with `interval_secs=60` shortcut), conditional-UPDATE race protection, Stripe Dashboard live-mode prep, post-deploy smoke. Sign-off block captures reconcile-counter values for retro.

**Files of interest** (verify before recommending):
- `server/src/services/reconcile.rs` (service + tests)
- `server/src/mappers/subscription.rs` (`ReconcileRow`, `list_for_reconcile`, `apply_reconcile_update`)
- `server/src/config/server.rs` (`ReconcileConfig`)
- `server/src/main.rs` (spawn site)
- `server/src/bin/set_subscription.rs` (`--reconcile-from-stripe` mode)
- `server/src/metrics/names.rs` (`ENTITLEMENT_RECONCILE_*`, `LABEL_FIELD`)
- `frontend/src/components/UpgradePage.vue` (2-tier layout)
- `docs/checklists/2026-05-track-4-release-readiness.md` (manual e2e + prod prep)

**Multi-replica note:** loop is single-process safe. When the deploy goes multi-replica, either wrap the pass in `pg_try_advisory_lock` or extract the loop into a dedicated CronJob. Not a v1 concern.
