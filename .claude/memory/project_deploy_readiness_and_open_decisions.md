---
name: project_deploy_readiness_and_open_decisions
description: "Pre-deploy assessment lives in docs/deployment-readiness.md; two platform decisions (Postgres, Redis) were left open on 2026-09-10 for the user to sleep on."
metadata: 
  node_type: memory
  type: project
  originSessionId: 0022e1bb-6627-4196-acb8-6586b29d9b35
  modified: 2026-09-10T20:30:49.410Z
---

**Read `docs/deployment-readiness.md` first on any "let's deploy" / "what's left before shipping" request.** Written 2026-09-10; it holds the blockers with file:line evidence, the verified runtime facts that gate serverless viability, the provisioning prerequisites, and the platform analysis.

Do not re-derive that analysis — several facts in it cost multiple searches and source reads to establish.

**Headline state (2026-09-10):**
- `AUDIT.md` docket is fully closed. Nothing outstanding is a code-quality defect.
- **Hard blockers:** `schema.sql` is 5 months stale and is the *only* DB bootstrap path (server never runs migrations); CI on `main` red since 2026-07-24 on `cargo-deny` **only** (RUSTSEC-2026-0204, crossbeam-epoch — code jobs all pass); no TLS in the topology, which silently blocks login because `cookie_secure = true`.
- **216 unticked UAT boxes across 5 checklists in `docs/checklists/`, zero ticked.** Stripe, monetisation, co-editor sharing and legal were all written to be run against a real environment and never have been. This is the point of building staging.
- Legal pages are still literal `"coming soon"` placeholders — blocks production, not staging.

**Decided:** Cloud Run (not a VM) in `europe-west1`; migrations at startup replacing `schema.sql`; reconcile loop ported to an HTTP endpoint driven by Cloud Scheduler with `interval_secs = 0`; Cloudflare Access for gating; domain from Porkbun with nameservers pointed at Cloudflare immediately; Resend for SMTP.

**Still open — the user is sleeping on both (as of 2026-09-10), resume here next session:**
1. **Postgres** — Cloud SQL (~$9/mo, same-cloud, native Cloud Run Unix-socket integration with no VPC connector fee) vs Neon (free, branching, but **AWS/Azure only — not on GCP**, so every query is cross-cloud to Frankfurt). His hesitation is explicitly about taking on a known recurring cost when free options exist.
2. **Redis** — Redis Cloud free tier (real Redis Enterprise, native pub/sub guaranteed, runs in GCP europe-west1, 30 MB / **30 connections** ≈ a dozen concurrent users) vs Upstash (unresolved whether `SUBSCRIBE` works over their native TCP endpoint; needs a live test he'd have to create the account for) vs Memorystore (~$36/mo, no constraints).

**Hard constraint he stated:** staging and production must use the **same mechanism**, not two different ones. This is what rules out a Redis sidecar container, which would otherwise be free and works fine at `max-instances=1`.

**How to apply:** open the doc, confirm whether the two open decisions have been made, and only then write the implementation plan. He asked for the plan to be deferred until they are. Related: [[project_dev_db_migration_drift]] (same manual-sync failure mode as `schema.sql`), [[user_gcp_cloud_run_experience]], [[reference_deep_dive_plan]].
