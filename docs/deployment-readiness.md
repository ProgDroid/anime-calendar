# Deployment Readiness — Staging & Production

**Date:** 2026-09-10
**Status:** Analysis complete, platform decisions partly open (see §5)
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

### B-2. CI on `main` is red

Last green run: 2026-06-12. The 2026-07-24 run failed on **`cargo-deny` only** —
Frontend, Backend, OpenAPI Validation and Frontend E2E all passed, so this is a
dependency advisory, not a code regression.

- `RUSTSEC-2026-0204` — crossbeam-epoch 0.9.18, invalid pointer dereference in
  the `fmt::Pointer` impl for `Atomic`/`Shared`. Transitive.
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
- Set `environment = "staging"`, not `"production"` — the `set_subscription`
  dev CLI refuses to run under production, and it is needed to shortcut
  §6 and §8 of the Track 4 checklist.
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

- *Redis Cloud free tier* — real Redis Enterprise so native-protocol pub/sub is
  unambiguous; **available in GCP `europe-west1`** so it co-locates with Cloud
  Run; free forever, no card. 30 MB (ample — TTL'd cache blobs only) and
  **30 connections**, which per §2's budget holds ~a dozen concurrent users.
  Upgrade path is the same product at ~$5/mo (Essentials, 250 MB).
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
- `lcov.info` untracked and unignored at repo root.
- Leftover agent worktrees under `.claude/worktrees/` consuming disk.
- `.semgrep/` added to `.gitignore` 2026-09-10 — it had written
  `guardian.yml` (OAuth access **and** refresh token) untracked and unignored.
  Nothing semgrep-related is in git history; verified with `git ls-files .semgrep/`.
