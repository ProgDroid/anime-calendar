# Deployment Readiness — Staging & Production

**Date:** 2026-09-10 · **Revised:** 2026-09-22
**Status:** Analysis complete, platform decisions partly open (see §5)

> **2026-09-22 revision.** Cloud sessions implemented Tasks 1–8 of
> `docs/superpowers/plans/2026-04-20-deployment.md` without access to this
> document (it had not been pushed), so the plan's April-era platform choices —
> `us-central1`, Neon for staging, Upstash for Redis — were followed. The code
> that resulted is **vendor-neutral** and nothing needs undoing; the plan has
> been reconciled against this document and carries a *Decisions of record*
> table. Corrections and new findings from that review are marked inline below:
> B-2 (three advisories, not one), §2 (the Redis budget is an artifact; Postgres
> pooling is unbudgeted), §3 (the `environment = "staging"` workaround is
> withdrawn), §5 (Redis Cloud tier evidence), §6 (`lcov.info` is now tracked).
**Context:** Pre-deploy assessment. Goal is a gated staging environment (Stripe
test mode, a handful of friend testers) followed by production.

The audit docket (`AUDIT.md`) is fully closed — every CRITICAL/HIGH/MEDIUM/LOW
from the 2026-05-07 sweep and 2026-06-11 follow-up is resolved. **Nothing below
is a code-quality defect.** The gaps are deployment plumbing that has never
existed, and manual verification that has never been run.

---

## 1. Hard blockers

### B-1. `schema.sql` is five months stale and is the only DB bootstrap path

`docker-compose.yml` mounts `./schema.sql` into
`/docker-entrypoint-initdb.d/`. The server does **not** run migrations at
startup — grepping the whole server crate for `migrat` yields only an unrelated
comment in `entity/subscription.rs:29`.

`schema.sql` last changed at `a85e4ac` (soft-deletes, April) and defines
4 tables. There are 21 migrations.

| Table | Present in schema.sql |
|---|---|
| `users`, `calendars`, `calendar_items`, `user_settings` | yes |
| `refresh_tokens` | **no** |
| `password_reset_tokens` | **no** |
| `subscriptions` | **no** |
| `stripe_events` | **no** |
| `calendar_editors` | **no** |
| `calendar_invitations` | **no** |

A fresh `docker compose up` therefore produces an April-era database. Login
refresh, password reset, billing and all of co-editor sharing fail at runtime.

The file's own header claims it is *"kept in sync manually whenever a migration
changes the schema"* — that manual sync stopped around migration 4 of 21. This
is the failure mode the manual-sync convention guarantees eventually.

**Decision taken:** run `sqlx::migrate!` at server startup and drop the
`schema.sql` mount. Migrations become the single source of truth so the drift
cannot recur. Note `sqlx` is already built with the `migrate` feature
(`server/Cargo.toml:42`). This is **mandatory** under Cloud Run, where there is
no host to shell into.

> **Contested, 2026-09-22.** The deployment plan instead runs `sqlx migrate run`
> from a GitHub Actions step before each deploy. That is not obviously worse —
> it runs exactly once per deploy rather than racing across cold-starting
> instances — but **the two documents currently assert different things, which
> is how the `schema.sql` drift started.** Settle it and record the answer here:
> Task 20 of the plan.
>
> Two facts established the same day. (a) The plan's three migration commands
> pointed at `server/migrations`, **which does not exist** — the 21 migrations
> are at `./migrations`. Every deploy would have failed on it. Corrected in the
> plan. (b) Applying all 21 in order to a fresh PostgreSQL 16 container
> succeeds: 21 applied, 0 failures, 11 tables. **The migration set is sound from
> zero**; the drift is a property of `schema.sql` alone.

### B-2. CI on `main` is red

Last green run: 2026-06-12. Still **`cargo-deny` only** — Frontend, Backend,
OpenAPI Validation and Frontend E2E all pass, so this is a dependency advisory,
not a code regression.

**Updated 2026-09-22 against CI run `35268609234` (2026-09-17): three advisories
now, not one.** Two arrived after this document was first written.

- `RUSTSEC-2026-0204` — crossbeam-epoch 0.9.18, invalid pointer dereference in
  the `fmt::Pointer` impl for `Atomic`/`Shared`. Transitive. *(The original.)*
- `RUSTSEC-2026-0258` — **h2, unbounded empty DATA frames. A remote DoS**, and
  the only one of the three that genuinely matters once this is publicly
  reachable. **New.**
- `RUSTSEC-2026-0285` — rustls, TLS 1.3 handshake messages incorrectly accepted
  across encryption level boundaries. **New.**
- Yanked `spin` crate (warning only).

Fix is a targeted `cargo update`. `deny.toml` already carries a documented
ignore convention — do **not** add to `ignore` without a corresponding entry
here (existing precedent: `RUSTSEC-2023-0071` for rsa via google-oauth).

### B-3. No TLS anywhere in the topology — this hard-blocks login

`frontend/nginx.conf` listens on port 80 only, no certs, no redirect.
`config.docker.toml.dist` sets `cookie_secure = true`, and
`Config::validate` (`server/src/config/server.rs:475`) refuses to start when
`environment = "production"` and `cookie_secure = false`.

Over plain HTTP the browser silently discards the session cookie. Nobody can log
in, and **there is no error message pointing at the cause** — this will burn an
hour if hit unprepared.

### B-4. Stripe webhook needs a public HTTPS URL

Downstream of B-3, and it is the thing staging exists to exercise.

---

## 2. Verified runtime facts (established by reading source, 2026-09-10)

These were checked because they determine whether a serverless deployment is
viable at all. Recording them so they need not be re-derived.

### Redis holds no durable state — it is a pure cache

`subscribe:{token}` carries a TTL (`export_ttl_seconds = 300`) and
`controllers/calendar.rs:538` implements a documented lazy-regeneration
fallback. The durable blob is `frozen_subscribe_ics` in Postgres. **Losing Redis
costs latency, not data.** This is what makes an ephemeral or small Redis
tolerable.

### Event fan-out is Redis Pub/Sub, not in-process broadcast

`services/calendar_events.rs:89` publishes to channel `cal:{calendar_id}`.
Multi-instance deployment was designed for from the start. The only per-instance
state is `SseConnectionTracker` (`sse_connection_tracker.rs:26`,
`Mutex<HashMap<i32, usize>>`), which enforces only the per-user connection cap
(`sse_max_connections_per_user = 8`). Across N instances that cap
under-enforces — a soft limit, not a correctness bug.

### SSE survives request timeouts

`frontend/src/composables/usePresence.ts:59` uses native `EventSource`, and
lines 64–69 explicitly document relying on browser auto-retry. A platform
request cap (e.g. Cloud Run's 60 min) produces a transparent reconnect.

### Redis connections are per-channel, not per-client

`redis_pubsub.rs:150` fast-paths to an existing `tokio::broadcast::Sender`;
only the first subscriber for a channel opens a dedicated connection (`:243`).
Reaping works: `:232` breaks the listener at `receiver_count() == 0`, `:321`
removes the map entry. No leak.

`Cache` (`cache.rs:44`) and `PresenceService` (`presence.rs:50`) each hold a
single `get_multiplexed_async_connection()`.

**Connection budget:**

```
fixed:                ~3   (cache + presence + publisher)
per viewed calendar:   2   (cal:{id}, presence:{id})
per active viewer:     1   (kick:{actor_id})
```

5 calendars / 10 viewers = ~23 connections. 10 calendars / 20 viewers = ~43.

> **Correction, 2026-09-22 — this budget is an implementation artifact, not a
> property of the app, and it should not be treated as an input to a vendor
> decision.** `redis_pubsub.rs:235-237` opens a fresh `Client::open` +
> `get_async_pubsub()` **per channel**. Redis pub/sub permits one connection to
> `SUBSCRIBE` to many channels; the per-channel connection is a choice.
> Multiplexing onto one shared subscriber collapses the whole table above to
> **~4 connections flat, regardless of load** — see Task 16 of the deployment
> plan. With that done, a free tier's connection cap stops being the binding
> constraint and throughput (e.g. Redis Cloud free: 100 ops·s⁻¹, 5 GB·mo⁻¹)
> becomes the limit that matters instead.

### Postgres connections are unbudgeted, and this is the sharper constraint

Established 2026-09-22; missing from the original assessment, which costed Redis
but not Postgres.

`main.rs` calls `Database::new` **15 times**, each building an independent
`PgPool`. Verified against sqlx 0.8.6 source
(`sqlx-core-0.8.6/src/pool/options.rs:143-166`): `max_connections: 10`,
`min_connections: 0`, `idle_timeout: 10 min`, `max_lifetime: 30 min`,
`test_before_acquire: true`.

So idle settles near zero — but the **ceiling is 150 connections per instance**,
and the deployment plan runs production at `--max-instances=5`. A `db-f1-micro`
tops out around 25. Nothing caps this today.

Two consequences: any Postgres sizing must be done against
`max_connections × 15 × max-instances`, and an explicit cap belongs in
`mappers/database.rs::Database::new` (one line, covers all 15 sites) before
anything is provisioned. `test_before_acquire` already defaults to `true`, which
is the behaviour a suspend-happy serverless Postgres needs; `idle_timeout` at 10
minutes is the value that would need lowering for one.

### The reconcile loop is the one genuine serverless obstacle

`services/reconcile.rs:440` spawns an in-process `tokio::time::interval`,
hourly, guarded by `pg_try_advisory_lock`. Platforms that throttle CPU between
requests (Cloud Run's default) will never fire it.

**The codebase already anticipated this.** `ReconcileConfig::interval_secs = 0`
disables the loop, documented as *"used in tests and in deployments that don't
want background work"*, and `run_pass` is already `pub async fn`
(`reconcile.rs:149`). The port is: expose `POST /internal/reconcile`, protect it
with platform OIDC service-account auth, drive it from Cloud Scheduler, set
`interval_secs = 0`. The advisory lock stays and keeps guarding overlap.

### Redis client has no TLS support

`cache.rs:38` and `presence.rs:41` hardcode `redis://`, and the crate is built
with `features = ["aio", "tokio-comp"]` (`server/Cargo.toml:37`) — no TLS
feature. **Any hosted Redis requires `rediss://`**, so that path costs a TLS
feature plus a config flag.

---

## 3. Configuration that must change for any real host

- `allowed_origins`, `app_base_url`, `frontend_url` are all `http://localhost`
  in `config.docker.toml.dist`.
- **SMTP is unconfigured.** `services/email.rs` no-ops with a WARN when
  `smtp.host` is empty (see doc comments at `:76`, `:104`, `:140`). This
  silently kills password reset, email verification **and co-editor
  invitations** — friend testers cannot self-serve invites.
- **Google OAuth consent screen** — if still in Testing mode, only whitelisted
  accounts can sign in. Testers must be listed, or the app published.
- `POSTGRES_PASSWORD: change-me` in `docker-compose.yml`.
- ~~Set `environment = "staging"`, not `"production"` — the `set_subscription`
  dev CLI refuses to run under production, and it is needed to shortcut
  §6 and §8 of the Track 4 checklist.~~

  **Superseded 2026-09-22.** This workaround trades a real safety check for a
  comp mechanism and should not be used on anything publicly reachable:
  `Config::validate` (`config/server.rs:619`) only enforces
  `cookie_secure = true` *when `environment == "production"`*, so setting
  `"staging"` to placate the CLI silently disarms the check that stops the
  session cookie being discarded over plain HTTP — the exact failure mode B-3
  describes, now with no guard rail.

  The underlying need is real and newly explicit: the owner wants to use the
  product himself without paying (self-subscribing is redundant) and to comp a
  few friends. Entitlement is just a `tier` column on `subscriptions`, so the
  capability exists; the only non-Stripe writer is the `set_subscription` CLI,
  guarded at `bin/set_subscription.rs:287` against `APP_ENV=production|prod`.
  **Needed: a comp path that works with `environment` set truthfully** — either
  an explicit override flag on the CLI (so bypassing the guard stays a visible,
  deliberate act) or an admin-only grant endpoint. Task 18 of the deployment
  plan.
- No Postgres backup story exists.

---

## 4. Never-run manual verification

216 unticked boxes across 5 checklists, zero ticked. These were written to be
run against real Stripe test mode in a browser — which is what staging is for.

| Checklist | Open |
|---|---|
| `2026-05-track-4-release-readiness.md` | 71 |
| `2026-05-monetisation-uat.md` | 65 |
| `2026-05-monetisation-stripe-price-update.md` | 39 |
| `2026-05-co-editor-sharing-uat.md` | 23 |
| `2026-05-legal-pre-release.md` | 18 |

**Legal blocks production, not staging.** `/privacy` and `/terms` render
literally `"Privacy policy coming soon."`
(`frontend/src/locales/en.json:542,546`). Fine for friends on Stripe test mode;
not fine for taking real money from strangers.

---

## 5. Platform analysis

### Decided

- **Cloud Run**, not a VM. Owner has significant prior Cloud Run experience, so
  the learning-curve cost that would favour a VM does not apply. The three
  objections were investigated: SSE is a non-issue (§2), Redis statelessness
  makes small/ephemeral caches tolerable (§2), and the reconcile loop has a
  small, idiomatic port (§2).
- **Region: `europe-west1`** (Belgium). Cloud Run pricing is near-flat across
  regions and scale-to-zero removes any idle-cost argument, so the US free-tier
  reasoning that would favour `us-central1` for a VM does not apply. Deciding
  factors: (a) **GDPR** — the unticked legal checklist requires naming a legal
  basis and Data Controller; keeping EU user data in the EU makes that
  paragraph short instead of an international-transfer justification;
  (b) latency ~10–20 ms vs ~120 ms from Europe, which matters for SSE presence;
  (c) every needed service is available there today.
- **Gating: Cloudflare Access** (free ≤50 users, email OTP, per-person
  revocation, hides origin IP).
- **Domain: Porkbun**, nameservers pointed at Cloudflare immediately. ICANN
  locks a newly-registered domain from *transfer* for 60 days, but pointing
  nameservers is free and instant — registrar transfer can follow later.
- **Email: Resend** — 3,000/mo free, SMTP credentials drop into the existing
  `[smtp]` block with no code change, no sandbox-approval delay (unlike SES).

Domain is genuinely step zero: it blocks TLS certificates, email DKIM/SPF, and
Google OAuth (which rejects bare IPs as authorized origins).

### Open — owner sleeping on these (2026-09-10)

**Postgres.** Torn between known-cost stability and free options.

- *Cloud SQL* — ~$9/mo, always on. Native Cloud Run integration over a Unix
  socket via `--add-cloudsql-instances`, **no Serverless VPC Access connector**
  (which would otherwise cost ~$8/mo and eat any savings). Same cloud, same
  region, automated backups + PITR. Same product staging and prod.
- *Neon* — **does not run on GCP** (AWS and Azure only; Azure regions
  deprecated, no new projects, free-tier projects idle 90+ days deleted from
  2026-10-05). Nearest European region is `aws-eu-central-1` (Frankfurt), so
  every query is cross-cloud over the public internet: ~10–15 ms RTT each, plus
  GCP egress, plus two vendors. Free, and its copy-on-write **branching** is
  genuinely attractive for rehearsing migrations against prod-shaped data.
  Caveat: sqlx's pool holds idle connections, which either keeps Neon awake
  (defeating scale-to-zero) or gets killed on suspend — needs `idle_timeout`
  below Neon's suspend threshold plus `test_before_acquire`.
- *Supabase* — ruled out: free tier pauses after 7 days idle and needs a
  **manual** unpause, which is recurring friction on a weekly-visited staging box.

**Redis.** Same — sleeping on it.

- *Redis Cloud free tier* — real Redis Enterprise, **available in GCP
  `europe-west1`** so it co-locates with Cloud Run; free forever, no card.
  30 MB (ample — TTL'd cache blobs only) and **30 connections**.

  **Evidence added 2026-09-22 ([Essentials plan details][rc-plans],
  [upgrade docs][rc-upgrade]):**

  | Plan | Connections | Throughput | Bandwidth/mo |
  |---|---|---|---|
  | 30 MB (free) | 30 | 100 ops·s⁻¹ | 5 GB |
  | 250 MB (~$5) | **256** | 1,000 ops·s⁻¹ | 100 GB |
  | 1 GB | 1,024 | 2,000 ops·s⁻¹ | 200 GB |

  The free→paid jump is 30 → 256 connections, and **the upgrade is in place**:
  *"When you change your plan, your data and endpoints are not disrupted"*, with
  no availability impact. Same connection string, no redeploy. That materially
  changes the risk calculus — outgrowing the free tier costs a console click and
  a card, not a migration under pressure.

  **Not settled:** the Essentials pages state no pub/sub restriction, but they
  also never affirm pub/sub *for Essentials specifically*. That is the same
  evidential gap that left Upstash unresolved, and it deserves the same
  treatment — a live `SUBSCRIBE` against a real free database before committing.
  The test is nearly free here (no card). One caveat ruled out: Redis
  Enterprise's clustered-pub/sub sharding caveat does not apply, as Essentials
  free is single-shard.

  [rc-plans]: https://redis.io/docs/latest/operate/rc/subscriptions/view-essentials-subscription/essentials-plan-details/
  [rc-upgrade]: https://redis.io/docs/latest/operate/rc/subscriptions/view-essentials-subscription/
- *Upstash* — **UNRESOLVED**: whether `SUBSCRIBE` works over their *native TCP*
  endpoint. Their compatibility page lists Pub/Sub as a category and says both
  TCP and REST are supported but never states it for the native protocol; their
  troubleshooting page documents `ERR max concurrent connections exceeded`,
  advises closing connections promptly, and recommends the REST client "to avoid
  persistent connections". Architecturally the wrong shape regardless — this app
  holds a `SUBSCRIBE` open for a user's whole browser session. Settling it needs
  a live test against a real free database (account creation is manual).
- *Memorystore* — ~$36/mo, no constraints, exact prod rehearsal. 4x the cost of
  everything else combined.

Owner constraint driving both: **staging and production should use the same
mechanism**, not two different ones. This is what rules out a Redis sidecar
container (which would otherwise be $0 and works fine at `max-instances=1`).

### Indicative cost

Staging with Cloud SQL + Redis Cloud free ≈ **$9/mo**. Cloud Run at this traffic
is ~$0–2, Cloud Scheduler free for 3 jobs, Artifact Registry ~$0.10.
Production adds a second Cloud Run service and a ~$5 Redis tier.

**Production cliff to be aware of:** real users mean the Redis free tier's 30
connections and any `max-instances=1` assumption both stop holding.

---

## 6. Cosmetic / housekeeping

- 9 stale TODOs in source, all low-stakes; one (`controllers/user.rs:431`,
  "refactor frontend, it's a mess rn") predates the redesign that already shipped.
- ~~`lcov.info` untracked and unignored at repo root.~~ **Worse as of `dab3a7c`
  (2026-09-22): it is now *tracked*** — 22,092 lines of coverage artifact
  committed. The intended fix was a `.gitignore` entry; this needs
  `git rm --cached lcov.info` plus the ignore line. (The `.semgrep/` entry added
  in the same commit is correct and wanted.)
- Leftover agent worktrees under `.claude/worktrees/` consuming disk.
- `.semgrep/` added to `.gitignore` 2026-09-10 — it had written
  `guardian.yml` (OAuth access **and** refresh token) untracked and unignored.
  Nothing semgrep-related is in git history; verified with `git ls-files .semgrep/`.
