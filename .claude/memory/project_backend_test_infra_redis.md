---
name: backend-full-green-test-run-needs-postgres-and-redis-on-2435
description: "cargo test -p server --lib needs live Postgres + Redis; includes a copy-paste recipe for throwaway containers, which sidesteps the dev-DB migration drift entirely."
metadata: 
  node_type: memory
  type: project
  originSessionId: 1b126c7d-2485-4fb6-9437-7b543abb4528
  modified: 2026-09-22T13:43:41.569Z
---

A fully-green `cargo test -p server --lib` (and `--workspace --lib`) needs TWO live services: Postgres AND Redis. Both come from env vars whose defaults only suit the original dev box:

- **Postgres** — `DATABASE_URL`. `server/src/test_helpers.rs:50,58` `.expect()`s it; no default.
- **Redis** — `REDIS_HOST` (default `aegyptvault.local`) and `REDIS_PORT` (default **2435**). Three `for_tests()` helpers read them: `cache.rs:24-31`, `redis_pubsub.rs:326,342`, `services/presence.rs:164`, each ending in `.expect("test Redis must be reachable")`.

**Diagnostic shortcut:** if the ONLY failing test is `controllers::calendar::integration_tests::subscribe_feed_free_no_blob_no_history_returns_404` panicking at `cache.rs:31`, that is Redis-not-running — **not a code regression**. Confirmed 2026-06-11 (audit Step 5): 370 passed / 1 failed solely because nothing was listening on 2435.

## Throwaway-container recipe (measured green 2026-09-22: 401 passed / 0 failed)

Prefer this over the dev DB — a fresh container has **no migration drift**, so it sidesteps [[project_dev_db_migration_drift]] completely. Port 5432 is often already taken by another project's container on this machine, hence 5433.

```bash
docker run -d --name ac-test-pg    -p 5433:5432 \
  -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=anime_calendar postgres:16
docker run -d --name ac-test-redis -p 6390:6379 redis:7-alpine
sleep 6

# Migrations live at ./migrations (21 of them) — NOT server/migrations.
# The installed sqlx-cli has no postgres driver ('no driver found for URL scheme
# "postgres"'), so apply them through psql rather than reinstalling sqlx-cli.
for f in $(ls migrations/*.sql | sort); do
  docker exec -i ac-test-pg psql -v ON_ERROR_STOP=1 -U postgres -d anime_calendar < "$f"
done

export DATABASE_URL="postgres://postgres:postgres@127.0.0.1:5433/anime_calendar"
export REDIS_HOST=127.0.0.1 REDIS_PORT=6390
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server --lib -- --test-threads=4

docker rm -f ac-test-pg ac-test-redis
```

`_sqlx_migrations` bookkeeping is **not** needed: the server never calls `sqlx::migrate!` at startup, so the tests want the schema, not the ledger. That is what makes the raw-psql loop safe here.

**How to apply:** confirm both services are up before calling a backend failure a regression. Never pipe the test command through `tail` — see [[feedback_test_command_no_pipe]].
