---
name: project_deploy_readiness_and_open_decisions
description: "START HERE on any deploy ask. CI is green, Tasks 1-8/15/18/19 shipped; blocked on ONE question from the user (environment shape) plus buying the domain. Task 16/14/17 are unblocked work."
metadata: 
  node_type: memory
  type: project
  originSessionId: 0022e1bb-6627-4196-acb8-6586b29d9b35
  modified: 2026-09-22T18:19:40.807Z
---

**Read `docs/deployment-readiness.md` first on any "let's deploy" / "what's left before shipping" request**, then `docs/superpowers/plans/2026-04-20-deployment.md` — specifically its **Decisions of record** and **Sequencing** tables at the top. Do not re-derive that analysis; several facts in it cost multiple source reads.

## NEXT SESSION PICKS UP HERE (state as of 2026-09-22)

**Blocked on the user, not on work.** One upstream question decides almost everything:

> **Is this a bounded pre-launch campaign, permanent infrastructure alongside production, or should there be only ONE environment?**

He has not answered it. It was put to him and he said he is not sure — *"it will depend how much maintenance or feature work I will do later"*. It determines the Postgres vendor, the Redis tier, and whether plan Tasks 9–12 build one environment or two. **Do not re-litigate Cloud SQL vs Neon before this is settled** — his hesitation was never about the comparison.

**Step zero, and no code moves it: buy the domain** (Porkbun, nameservers → Cloudflare immediately). It gates TLS (B-3), DKIM/SPF for Resend, and Google OAuth, which rejects bare IPs as authorized origins. B-3 and B-4 both wait on it.

**Unblocked work, in the recommended order** — none of it depends on the open decisions:
1. **Task 16 — Redis subscriber multiplexing.** Direct sibling of the pool work already shipped: `redis_pubsub.rs:235-237` opens one TCP connection per channel where one connection can carry many. Collapses the budget to ~4 flat. Recommended next.
2. **Task 14 — reconcile loop → Cloud Scheduler.** Under Cloud Run's default CPU throttling the hourly `tokio::time::interval` never fires and Stripe state silently stops reconciling. Needs `RECONCILE__INTERVAL_SECS=0` + an endpoint + an OIDC-authed job.
3. **Task 17 — Cloudflare Access + a smoke test that survives it.** The deploy's smoke test curls the public domain, which Access answers with a login redirect: a healthy service reported as failed.

**Also needs a decision, not work: Task 20 (migration strategy).** `docs/deployment-readiness.md` B-1 says migrate at startup; the plan's Task 12 migrates from CI. **The two documents currently contradict each other** — which is exactly how the `schema.sql` drift started. Settle it and write the answer in the readiness doc.

## Shipped 2026-09-22

- **Plan Tasks 1–8** (from the cloud-session branch, squashed): health endpoint, env-var config layering, Redis TLS + unified URL across all three consumers, Cloudflare origin middleware, cookie domain/`SameSite=Lax`, nginx envsubst template, `database.url` passthrough, swagger-ui behind a feature flag.
- **Task 19** — `cargo deny check` green with **zero** `deny.toml` ignores. The h2 0.3 advisory had no backport, so actix-web's `http2` feature was dropped to remove the crate; CLAUDE.md records why re-adding it needs care.
- **Task 15** — one Postgres pool per process, `max_connections` default 5. See CLAUDE.md.
- **Task 18** — `--i-know-this-is-production` comp path; the CLI also now honours `database.url` (it previously ignored it and could not reach a managed Postgres at all).
- **CI on `main` is GREEN** — all five jobs, first time since 2026-06-12. **B-2 is closed.**

## Decided (do not reopen)

Cloud Run in `europe-west1`; Cloudflare Access for gating; Porkbun domain; Resend for SMTP. **Hard constraint he stated: staging and production must use the same mechanism** — this is what rules out a Redis sidecar.

## The two vendor choices — sizing no longer constrains either

Both were stalled on numbers that turned out to be **properties of our own code, not the vendors'**. That is resolved; they are now pure cost-and-preference calls downstream of the environment-shape question.

- **Postgres** — Cloud SQL (~$9/mo, same cloud, Unix socket, no VPC connector fee) vs Neon (free, branching, but AWS/Azure only so every query is cross-cloud to Frankfurt). Task 15 made the budget one number that fits a `db-f1-micro`, so the cheapest tier is viable.
- **Redis** — leaning Redis Cloud free, not fully convinced. Measured: free 30 MB = 30 conns / 100 ops·s⁻¹; **250 MB (~$5) = 256 conns** / 1,000 ops·s⁻¹; upgrades leave *"data and endpoints not disrupted"*, so outgrowing free is a console click, not a migration. **Still unverified: that pub/sub works on Essentials** — documented as unrestricted, never documented as supported, which is the same gap that ruled out Upstash. Settle with a live `SUBSCRIBE` against a real free database (no card needed). Upstash is ruled out: wrong shape for a per-session `SUBSCRIBE`.

## Still open beyond the platform choices

216 unticked UAT boxes across 5 checklists in `docs/checklists/`; legal pages are literal `"coming soon"` (blocks production, not staging).

**How to apply:** put the environment-shape question first, confirm the domain is bought, then pick from the unblocked list. Related: [[project_dev_db_migration_drift]], [[user_gcp_cloud_run_experience]], [[reference_deep_dive_plan]], [[backend-full-green-test-run-needs-postgres-and-redis-on-2435]].
