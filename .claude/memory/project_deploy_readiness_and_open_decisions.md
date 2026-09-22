---
name: project_deploy_readiness_and_open_decisions
description: "Pre-deploy state: docs/deployment-readiness.md is the authority, the 2026-04-20 plan is now reconciled against it, and Postgres + Redis vendors are still the only open decisions."
metadata: 
  node_type: memory
  type: project
  originSessionId: 0022e1bb-6627-4196-acb8-6586b29d9b35
  modified: 2026-09-22T13:44:06.139Z
---

**Read `docs/deployment-readiness.md` first on any "let's deploy" / "what's left before shipping" request**, then `docs/superpowers/plans/2026-04-20-deployment.md` — specifically its **Decisions of record** and **Sequencing** tables at the top. Do not re-derive the analysis; several facts in it cost multiple source reads.

**Headline state (2026-09-22):**
- Tasks 1–8 of the plan are **shipped on `main`** (health endpoint, env-var config, Redis TLS + unified URL, Cloudflare origin middleware, cookie domain/SameSite, nginx envsubst, `database.url`). Verified: 401 backend tests pass, clippy clean, 21 migrations apply from zero.
- `AUDIT.md` docket fully closed. Nothing outstanding is a code-quality defect.
- **CI on `main` is red on `cargo-deny` only — now THREE advisories, not one.** `RUSTSEC-2026-0204` (crossbeam-epoch), `RUSTSEC-2026-0258` (**h2, remote DoS — the one that matters publicly**), `RUSTSEC-2026-0285` (rustls). Plan Task 19.
- 216 unticked UAT boxes across 5 checklists; legal pages still literal `"coming soon"` (blocks production, not staging).

**Decided:** Cloud Run in `europe-west1`; Cloudflare Access for gating; Porkbun domain with nameservers at Cloudflare; Resend for SMTP.

**Still open — the ONLY two, and only plan Tasks 9/10/11 depend on them:**
1. **Postgres** — Cloud SQL (~$9/mo) vs Neon (free, branching, but not on GCP). **His blocker is not the comparison, it's committing to recurring cost with an unknown user base** (stated 2026-09-22). Two reframes he has not resolved: whether staging is a bounded pre-launch campaign or permanent infrastructure, and whether to skip a second environment entirely (one Cloud Run service, Stripe test mode → live mode).
2. **Redis** — leaning Redis Cloud free, not fully convinced. **Measured 2026-09-22:** free 30 MB = 30 conns / 100 ops·s⁻¹; **250 MB (~$5) = 256 conns** / 1,000 ops·s⁻¹; and Redis documents that plan upgrades leave "data and endpoints not disrupted" — so outgrowing free is a console click, not a migration. Upstash is ruled out (wrong shape for a per-session `SUBSCRIBE`).

**Hard constraint he stated:** staging and production must use the **same mechanism**. Rules out a Redis sidecar.

**New requirement (2026-09-22):** he wants to use the product himself without paying, and comp some friends. `bin/set_subscription.rs:287` refuses to run under `APP_ENV=production|prod`, and the old workaround (`environment = "staging"`) disarms the `cookie_secure` check `Config::validate` only enforces under production. Needs a real comp path — plan Task 18.

**How to apply:** confirm the two decisions before touching Tasks 9–11; everything else is vendor-neutral and can proceed. Related: [[project_dev_db_migration_drift]], [[user_gcp_cloud_run_experience]], [[reference_deep_dive_plan]], [[backend-full-green-test-run-needs-postgres-and-redis-on-2435]].
