---
name: project_deploy_readiness_and_open_decisions
description: "START HERE on any deploy ask. One environment (no staging); migrations = CI applies + startup verifies; Tasks 1-8/15/16/18/19 shipped. Only remaining blocker is buying the domain. Task 14/20/17 are unblocked work."
metadata:
  node_type: memory
  type: project
  originSessionId: 30c24443-62ce-4076-9320-3334d0c8b47e
  modified: 2026-09-22T19:26:16.787Z
---

**Read `docs/deployment-readiness.md` first on any "let's deploy" / "what's left before shipping" request**, then `docs/superpowers/plans/2026-04-20-deployment.md` — specifically its **Decisions of record**, **Decisions taken 2026-09-22** and **Sequencing** tables at the top. Do not re-derive that analysis; several facts in it cost multiple source reads.

## NEXT SESSION PICKS UP HERE (state as of 2026-09-22 evening)

**One blocker remains and it is not code: buy the domain.** Porkbun, nameservers → Cloudflare immediately. It gates TLS (B-3), Resend DKIM/SPF, Google OAuth (which rejects bare IPs as authorized origins), and Tasks 9–11. **The owner wants to decide *which* domain before registering** — so the next conversation is domain choice, not a purchase.

**Unblocked work, in the recommended order:**
1. **Task 14 — reconcile loop → Cloud Scheduler.** Under Cloud Run's default CPU throttling the hourly `tokio::time::interval` never fires and Stripe state silently stops reconciling. Needs `RECONCILE__INTERVAL_SECS=0` + an endpoint + an OIDC-authed job. Steps 4–5 need GCP access; Steps 1–3 are pure code.
2. **Task 20 — implement the migration decision** (decided, not done). Also drops the stale `schema.sql` mount, a hard blocker for a fresh setup.
3. **Task 17 — Access + a smoke test that survives it.** Step 1 needs the domain; the smoke-test fix does not.

## Decided (do not reopen)

- **ONE environment — production only, no staging** (2026-09-22). Tasks 9–12 build one of everything. The *"staging and production must use the same mechanism"* constraint is **moot**; it is no longer an argument for anything. A Redis sidecar is still ruled out, now because that container *is* production and dies with the instance under scale-to-zero.
- **Task 20 — CI applies, startup verifies.** `sqlx migrate run` from the deploy workflow (runs once per deploy, no cold-start race) **plus** a startup check reading `sqlx::migrate!`'s applied-versions list *without applying* that refuses to serve on a mismatch. Both documents now say this; they previously contradicted each other.
- Cloud Run in `europe-west1`; Cloudflare Access for gating; Porkbun domain; Resend for SMTP.

## Two consequences the staging split was hiding — surfaced 2026-09-22, both still open

- **Legal is now on the critical path to the FIRST deploy.** It was filed as *"blocks production, not staging"*, and that split was doing real work: it let the 18 unticked boxes in `2026-05-legal-pre-release.md` wait. With one environment the deferral is gone.
- **Stripe mode is an open question.** Testers were going to be on test mode because staging was separate. Either run the single environment in test mode and cut over (discarding test-mode subscription state), or run live from day one and comp testers via Task 18. **Settle before Task 10 populates secrets.** Coupled to the legal question — answer them together.

## Shipped

- **2026-09-22 evening — Task 16 (Redis subscriber multiplexing).** One shared `PubSub` connection per process, `split()` into sink + stream, one driver task demuxing on `get_channel_name()`. **4 connections per replica at any load, 3 with no viewers** (opened lazily on first `subscribe()`). Reaping is `UNSUBSCRIBE`, plus a sweep on the subscribe slow path for channels that go quiet. Details in [[feedback_redis_pubsub_shared_subscriber]].
- **2026-09-22 — Plan Tasks 1–8** (cloud-session branch, squashed): health endpoint, env-var config layering, Redis TLS + unified URL, Cloudflare origin middleware, cookie domain/`SameSite=Lax`, nginx envsubst template, `database.url` passthrough, swagger-ui behind a feature flag.
- **Task 19** — `cargo deny check` green with **zero** ignores (actix-web's `http2` feature dropped to remove h2 0.3).
- **Task 15** — one Postgres pool per process, `max_connections` default 5.
- **Task 18** — `--i-know-this-is-production` comp path; CLI now honours `database.url`.
- **CI on `main` is GREEN.** B-2 closed.

## The two vendor choices — now pure cost/preference, but Postgres re-weighted

- **Postgres** — Cloud SQL (~$9/mo, same cloud, Unix socket, no VPC connector fee) vs Neon (free, branching, AWS/Azure only so every query is cross-cloud to Frankfurt). **The single-environment decision made Neon's branching *more* valuable, not less** — with no staging it is the only way to rehearse a migration against prod-shaped data. Live trade-off.
- **Redis** — **connections are no longer the constraint.** Task 16 puts the ceiling at 4/replica × 5 instances = 20, against the free tier's 30. **Throughput is now the binding limit and it is UNMEASURED**: free = 100 ops·s⁻¹ / 5 GB·mo⁻¹; 250 MB (~$5) = 1,000 ops·s⁻¹ / 100 GB, upgradeable in place with no endpoint change. **Also still unverified: that pub/sub works on Redis Cloud Essentials** — documented as unrestricted, never documented as supported. Settle with a live `SUBSCRIBE` against a real free database (no card needed). Upstash is ruled out.

## Still open beyond the above

216 unticked UAT boxes across 5 checklists in `docs/checklists/`; legal pages are literal `"coming soon"`.

**How to apply:** the domain conversation comes first; then pick from the unblocked list. Related: [[project_dev_db_migration_drift]], [[user_gcp_cloud_run_experience]], [[reference_deep_dive_plan]], [[backend-full-green-test-run-needs-postgres-and-redis-on-2435]], [[feedback_redis_pubsub_shared_subscriber]].
