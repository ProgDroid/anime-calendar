# Deployment Readiness — Production

**Date:** 2026-09-10 · **Revised:** 2026-09-22 (evening)
**Status:** B-2 closed (CI green). Task 16 done. The environment-shape question
is **settled — one environment, production only, no staging** — and Task 20 is
**settled — CI applies migrations, startup verifies them.** The single remaining
blocker is **buying the domain**, which no code can move and which the owner
wants to choose before registering. Unblocked work: plan Tasks 14, 20, 17.

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
**Context:** Pre-deploy assessment. ~~Goal is a gated staging environment (Stripe
test mode, a handful of friend testers) followed by production.~~ **Revised
2026-09-22: there is no staging.** The goal is one production environment, gated
by Cloudflare Access while it is shown to friend testers, then opened. The
friend-testing phase is a *policy* on one environment, not a second environment.

> **Consequence not yet decided — Stripe mode.** The original plan put friend
> testers on Stripe *test* mode, which was a property of staging being separate.
> With one environment there are two options and nobody has picked one: run the
> single environment in test mode until launch and cut over to live keys (a
> cutover that discards test-mode subscription state), or run live from day one
> and give testers paid features through the comp path (Task 18, shipped, which
> exists precisely for this). **Decide before Task 10 populates secrets** — that
> is the step where the choice becomes concrete.

Wherever the text below still says "staging", read it as production unless it is
explicitly contrasting the two; the sections revised this evening (header, B-1,
§2, §5) use the corrected framing.

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

**DECISION OF RECORD (settled 2026-09-22 evening, Task 20): CI applies,
startup verifies.** This supersedes the earlier "migrate at startup" decision
recorded here, and it is what the deployment plan now says too — the two
documents agreed to disagree for a fortnight, which is exactly the condition
that produced the `schema.sql` drift in the first place.

1. **`sqlx migrate run --source ./migrations` from the deploy workflow**, before
   the new revision takes traffic. It runs exactly once per deploy instead of
   racing across however many instances cold-start at the same moment — a race
   `_sqlx_migrations` handles correctly but noisily.
2. **A startup check that verifies and does not apply.** `sqlx::migrate!`
   exposes the applied-versions list without running anything; the server
   compares it to the embedded set and **refuses to serve on a mismatch**. This
   closes the one thing the CI-only approach gives up: a container started
   outside the workflow — a local `docker run`, a manual `gcloud run deploy` of
   an older image — otherwise gets no migration at all and serves traffic
   against a schema it cannot satisfy, silently.
3. **Drop the `schema.sql` mount from `docker-compose.yml` either way.** It is a
   hard blocker for anyone setting the project up fresh, independent of which
   strategy won.

`sqlx` is already built with the `migrate` feature (`server/Cargo.toml:42`), so
(2) needs no dependency change. **The decision is made; the work is not done** —
it is Task 20 of the plan.

> Two facts established 2026-09-22 that this decision rests on. (a) The plan's
> three migration commands pointed at `server/migrations`, **which does not
> exist** — the 21 migrations are at `./migrations`. Every deploy would have
> failed on it. Corrected in the plan. (b) Applying all 21 in order to a fresh
> PostgreSQL 16 container succeeds: 21 applied, 0 failures, 11 tables
> (re-verified 2026-09-22 evening). **The migration set is sound from zero**;
> the drift is a property of `schema.sql` alone.
>
> With the single-environment decision (below), local containers are the *only*
> migration rehearsal there is. That raises the stakes on actually running (b)
> before each migration ships rather than trusting it.

### ~~B-2. CI on `main` is red~~ — CLOSED 2026-09-22

**All five jobs green on run `35763755502`, the first fully passing `main` since
2026-06-12.** Frontend, Frontend E2E, OpenAPI Validation, cargo-deny and
Backend (format, clippy and `cargo nextest run --workspace --all-targets`).

Two things worth keeping from closing it:

- **A CI job's first step failing leaves every later step UNKNOWN, not passing.**
  The run before this one failed Backend on *Format check*, which is step 9 of
  22 — so build, clippy and tests never executed, and "only formatting failed"
  read like good news while actually meaning the backend was entirely
  unverified. Run `cargo fmt --all -- --check` before pushing; CLAUDE.md now
  lists the backend gate in CI's own order.
- **The step named "Clippy (deny warnings)" does not deny warnings.** It passes
  only `-W` flags and `RUSTFLAGS` is just the lld linker arg, so the ~23 clippy
  warnings currently in the server lib do not fail CI. The *behaviour* is
  intentional (see the `reference_clippy_command` memory: curated allow-list,
  not bare `-D warnings`); the *name* is stale and asserts enforcement that
  does not exist.

The original analysis follows, for the record.

### B-2. CI on `main` is red (historical)

Last green run: 2026-06-12. Still **`cargo-deny` only** — Frontend, Backend,
OpenAPI Validation and Frontend E2E all pass, so this is a dependency advisory,
not a code regression.

> **RESOLVED 2026-09-22.** `cargo deny check` reports `advisories ok, bans ok,
> licenses ok, sources ok`, with **nothing added to the `ignore` list**. Three
> of the four cleared by `cargo update`. The fourth could not: `actix-http` 3.x
> requires `h2 ^0.3`, and **0.3.27 is the last release of that line** — the fix
> is in 0.4.16 and no backport exists, so no dependency bump could reach it.
> It was removed instead, by dropping actix-web's `http2` feature. Verified
> from actix-web's source that the HTTP/2 path was unreachable anyway (plain
> `bind()` → `listen()` → `.tcp()`; h2 needs `bind_auto_h2c()` or ALPN on
> `bind_rustls`/`bind_openssl`, and TLS terminates at nginx/Cloudflare), and
> verified that `actix-cors`, `actix-web-lab` and `utoipa-swagger-ui` all
> declare actix-web with `default-features = false` so feature unification
> would not quietly re-enable it. Plan Task 19.

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

### ~~Redis connections are per-channel, not per-client~~ — FIXED 2026-09-22 (Task 16)

**Redis connections are now flat.** `redis_pubsub.rs` holds **one** shared
`PubSub` connection for the whole process, split into a sink
(`SUBSCRIBE`/`UNSUBSCRIBE`) and a stream, with a single driver task
demultiplexing messages by channel name into the existing per-channel
`tokio::broadcast` fan-out. The connection is opened lazily on the first
`subscribe()`, so a replica with no SSE viewers holds none at all.

`Cache` (`cache.rs:44`) and `PresenceService` (`presence.rs:50`) each hold a
single `get_multiplexed_async_connection()`.

**Connection budget (current):**

```
fixed:                 3   (cache + presence + publisher)
shared subscriber:     1   (only while ≥1 SSE viewer is connected)
per viewed calendar:   0
per active viewer:     0
```

**4 connections per replica at any load**, 3 with no viewers. At
`--max-instances=5` that is a ceiling of 20 — inside the free tier's 30 with
room to spare, and inside the 256 of the ~$5 tier by two orders of magnitude.

Asserted against Redis rather than against belief: the test
`many_channels_share_one_connection` diffs `CLIENT LIST TYPE pubsub` around five
`subscribe()` calls and requires the count to grow by exactly one.

**Historical (2026-09-10), kept because the vendor analysis below was written
against it:** the old code opened a fresh `Client::open` + `get_async_pubsub()`
per channel, giving `~3 fixed + 2/calendar + 1/viewer` — 5 calendars / 10 viewers
= ~23 connections, 10 calendars / 20 viewers = ~43. That was an implementation
artifact, never a property of the app, and it is gone.

> **Still unmeasured — Task 16 Step 4.** With connections no longer scaling per
> viewer, the binding limit moves to **throughput**: Redis Cloud free is
> 100 ops·s⁻¹ and 5 GB·mo⁻¹. Nothing here has measured the app's actual ops rate,
> so treat "comfortable" as an assumption, not a finding. Every published event
> still costs one `PUBLISH` plus the cache's own traffic; multiplexing changed
> the *connection* count, not the *message* count.

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

> **RESOLVED 2026-09-22 — the pools are now one pool.** `main.rs` builds a
> single `Database` and every mapper and service takes an Arc-backed clone. The
> per-instance budget is therefore a single configurable number,
> `database.toml`'s `max_connections` (env `DATABASE__MAX_CONNECTIONS`),
> defaulting to **5**.
>
> The sizing formula for choosing a Postgres instance is now simply:
>
> ```text
> max_connections × (Cloud Run max-instances) + headroom  ≤  server limit
> ```
>
> with headroom for migrations, the `set_subscription` CLI and a `psql`
> session. At the default that is four instances inside a `db-f1-micro`'s ~25
> with five to spare — so the smallest tier is now viable, which it was not
> before. **This removes connection count as a reason to prefer one vendor over
> another.**
>
> Capping the fifteen pools instead would *not* have sufficed: fifteen pools at
> even 2 connections each is 30, already past `db-f1-micro` before multiplying
> by instances. Plan Task 15.
>
> Note for whichever vendor is chosen: `test_before_acquire` already defaults to
> `true`, which is what a suspend-happy serverless Postgres needs; sqlx's
> 10-minute `idle_timeout` is the value that would want lowering for one
> (Neon autosuspends at 5).

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

~~**Legal blocks production, not staging.**~~ **Reclassified 2026-09-22: legal
blocks the only deploy there is.** `/privacy` and `/terms` render literally
`"Privacy policy coming soon."` (`frontend/src/locales/en.json:542,546`).

The old sentence continued *"Fine for friends on Stripe test mode; not fine for
taking real money from strangers"* — and that split was doing real work, because
it let the legal checklist wait until after a staging deploy. **With one
environment that deferral is gone.** The 18 unticked boxes in
`2026-05-legal-pre-release.md` now sit on the critical path to the *first*
deploy, not a later one, unless the Stripe-mode question above is answered with
"test mode until launch" — in which case they move behind the cutover instead.
The two questions are coupled; answer them together.

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
- **ONE environment: production, with no staging** (decided 2026-09-22). Tasks
  9–12 of the deployment plan build a single environment; nothing is duplicated.
  The consequences are worked through in the plan's *Decisions taken
  2026-09-22* section — the short version is that migrations are rehearsed
  against a local container rather than a staging box, the
  "same-mechanism-in-both-environments" constraint below is moot, and there is
  no soft launch: the first deploy is production.

Domain is genuinely step zero: it blocks TLS certificates, email DKIM/SPF, and
Google OAuth (which rejects bare IPs as authorized origins). **Not bought as of
2026-09-22** — the owner wants to settle which domain to register first.

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

~~Owner constraint driving both: **staging and production should use the same
mechanism**, not two different ones.~~ **Moot as of 2026-09-22** — there is only
one environment, so there is nothing for it to match. It was the argument that
ruled out a Redis sidecar container. **The sidecar stays ruled out anyway**, for
a different and now more direct reason: with no staging, that container *is*
production, and it dies with the instance under scale-to-zero, taking the cache
and every live `SUBSCRIBE` with it.

The single-environment decision also re-weights the Postgres choice rather than
settling it. Neon's copy-on-write branching was attractive for rehearsing
migrations against prod-shaped data; with no staging it becomes the **only** way
to do that, so it gains value here. Set against every query being cross-cloud to
Frankfurt. Live trade-off — see the plan.

### Indicative cost

Production with Cloud SQL + Redis Cloud free ≈ **$9/mo** — one Cloud Run service,
no second environment. Cloud Run at this traffic is ~$0–2, Cloud Scheduler free
for 3 jobs, Artifact Registry ~$0.10.

**Production cliff, which now arrives on day one** (there is no staging to meet
it first):

- ~~the Redis free tier's 30 connections~~ — **no longer the binding constraint.**
  Task 16 made connections flat at 4 per replica, so `--max-instances=5` tops out
  at 20 against a ceiling of 30. See §2.
- **Throughput is now the ceiling that matters**, and it is unmeasured: free is
  100 ops·s⁻¹ and 5 GB·mo⁻¹. The ~$5 tier lifts that to 1,000 ops·s⁻¹ / 100 GB
  with an in-place upgrade (same endpoint, no redeploy).
- any `max-instances=1` assumption still stops holding.

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
